use std::sync::Arc;

use librespot_protocol::playlist4_external::SelectedListContent;
use protobuf::Message;

use crate::shared::music::domain::{Follow, FollowKind, Playlist, Source, UserView};

use super::*;
use super::{session::*, tracks::*};

pub async fn library(
    client: &reqwest::Client,
    session: &Arc<Session>,
) -> Result<Vec<Playlist>, String> {
    let username = session.username();
    if username.is_empty() {
        return Err("Sign in to Spotify again".to_owned());
    }

    let mut playlists = vec![Playlist {
        id: format!("spotify:user:{username}:collection"),
        cover: Some(LIKED_COVER.to_owned()),
        source: Source::Spotify,
        ..Default::default()
    }];
    let liked = {
        let client = client.clone();
        let session = session.clone();
        let username = username.clone();
        tokio::spawn(async move { liked_count(&client, &session, &username).await })
    };

    let mut from = 0usize;

    loop {
        let path = format!(
            "/playlist/v2/user/{username}/rootlist?decorate=revision,attributes,length,owner,status_code&from={from}&length=200"
        );
        let bytes = get_bytes(client, session, &path).await?;
        let content = SelectedListContent::parse_from_bytes(&bytes)
            .map_err(|error| format!("spotify library: {error}"))?;
        let Some(contents) = content.contents.into_option() else {
            break;
        };

        let count = contents.items.len();
        for (item, meta) in contents.items.iter().zip(contents.meta_items.iter()) {
            let uri = item.uri();
            let Some((_, id)) = uri.rsplit_once(":playlist:") else {
                continue;
            };

            if !matches!(meta.status_code(), 0 | 200) {
                continue;
            }

            let attributes = meta.attributes.get_or_default();
            let name = attributes.name.clone().unwrap_or_default();
            let cover = attributes.picture.as_deref().and_then(picture);
            let owner = non_empty(meta.owner_username());

            playlists.push(Playlist {
                id: format!("spotify:playlist:{id}"),
                name,
                subtitle: owner.clone(),
                cover,
                owner,
                source: Source::Spotify,
                track_count: u32::try_from(meta.length()).unwrap_or_default(),
                ..Default::default()
            });
        }

        if count == 0 || !contents.truncated() {
            break;
        }

        from += count;
    }

    playlists[0].track_count = liked.await.unwrap_or(0);
    enrich_covers(client, session, &mut playlists).await;
    Ok(playlists)
}

/// The rootlist carries no entry for Liked Songs, so its size comes from the
/// user's collection list, the same shape as a playlist.
pub(super) async fn liked_count(
    client: &reqwest::Client,
    session: &Arc<Session>,
    username: &str,
) -> u32 {
    let path = format!(
        "/playlist/v2/user/{username}/collection?decorate=revision,attributes,length&from=0&length=1"
    );
    let Ok(bytes) = get_bytes(client, session, &path).await else {
        return 0;
    };

    length(&bytes)
}

pub(super) fn length(bytes: &[u8]) -> u32 {
    let Ok(content) = SelectedListContent::parse_from_bytes(bytes) else {
        return 0;
    };

    u32::try_from(content.length.unwrap_or(0)).unwrap_or_default()
}

/// Liked Songs is a user-scoped context, not a playlist in the rootlist, so it
/// is resolved through the same endpoint a search uses.
pub async fn liked(client: &reqwest::Client, session: &Arc<Session>) -> Result<Playlist, String> {
    let username = session.username();
    if username.is_empty() {
        return Err("Sign in to Spotify again".to_owned());
    }

    let id = format!("spotify:user:{username}:collection");
    let uris = context_uris(client, session, &id).await?;
    let songs = tracks(client, session, &uris).await;
    let track_count = songs.len() as u32;
    Ok(Playlist {
        id,
        cover: Some(LIKED_COVER.to_owned()),
        source: Source::Spotify,
        songs,
        track_count,
        ..Default::default()
    })
}

pub(super) async fn context_uris(
    client: &reqwest::Client,
    session: &Arc<Session>,
    uri: &str,
) -> Result<Vec<String>, String> {
    context_pages(client, session, uri, usize::MAX).await
}

/// Reads a context page by page. A radio context never reports the end, so the
/// caller bounds how many pages it follows.
pub(super) async fn context_pages(
    client: &reqwest::Client,
    session: &Arc<Session>,
    uri: &str,
    pages: usize,
) -> Result<Vec<String>, String> {
    let mut uris = Vec::new();
    let mut path = format!("/context-resolve/v1/{uri}");
    let mut read = 0usize;
    loop {
        let response = authorized(session, "spotify context", |access| {
            client
                .get(format!("{SPCLIENT}{path}"))
                .header(reqwest::header::AUTHORIZATION, format!("Bearer {access}"))
        })
        .await?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!("spotify context answered {status}"));
        }

        let value = response
            .json()
            .await
            .map_err(|error| format!("spotify context: {error}"))?;

        track_uris(&value, &mut uris);
        read += 1;
        if read >= pages {
            break;
        }

        let Some(next) = next_page(&value) else {
            break;
        };

        path = format!("/{}", next.trim_start_matches("hm://"));
    }

    Ok(uris)
}

pub(super) fn track_uris(value: &serde_json::Value, uris: &mut Vec<String>) {
    let pages = value.get("pages").and_then(serde_json::Value::as_array);
    let values: Vec<&serde_json::Value> = pages
        .map(|pages| pages.iter().collect())
        .unwrap_or_else(|| vec![value]);

    for page in values {
        let Some(tracks) = page.get("tracks").and_then(serde_json::Value::as_array) else {
            continue;
        };

        for track in tracks {
            if let Some(uri) = track
                .get("uri")
                .and_then(serde_json::Value::as_str)
                .filter(|uri| uri.starts_with("spotify:track:"))
            {
                uris.push(uri.to_owned());
            } else if let Some(uri) = track
                .get("gid")
                .and_then(serde_json::Value::as_str)
                .and_then(track_gid)
            {
                uris.push(uri);
            }
        }
    }
}

/// A context page may carry the track as its raw 16-byte gid instead of a uri.
fn track_gid(gid: &str) -> Option<String> {
    use base64::Engine as _;
    let bytes = base64::engine::general_purpose::STANDARD.decode(gid).ok()?;
    (bytes.len() == 16).then(|| format!("spotify:track:{}", hex_to_base62(&hex(&bytes))))
}

pub(super) fn next_page(value: &serde_json::Value) -> Option<&str> {
    value
        .get("pages")
        .and_then(serde_json::Value::as_array)
        .and_then(|pages| {
            pages.iter().rev().find_map(|page| {
                page.get("next_page_url")
                    .and_then(serde_json::Value::as_str)
            })
        })
        .or_else(|| {
            value
                .get("next_page_url")
                .and_then(serde_json::Value::as_str)
        })
}

pub(super) async fn enrich_covers(
    client: &reqwest::Client,
    session: &Arc<Session>,
    playlists: &mut [Playlist],
) {
    let mut set = tokio::task::JoinSet::new();

    for (index, playlist) in playlists.iter().enumerate().take(32) {
        if playlist.cover.is_some() {
            continue;
        }

        let Some(id) = playlist
            .id
            .strip_prefix("spotify:playlist:")
            .map(str::to_owned)
        else {
            continue;
        };

        let client = client.clone();
        let session = session.clone();
        set.spawn(async move { (index, first_cover(&client, &session, &id).await) });
    }

    while let Some(result) = set.join_next().await {
        if let Ok((index, Some(cover))) = result
            && let Some(playlist) = playlists.get_mut(index)
        {
            playlist.cover = Some(cover);
        }
    }
}

pub(super) async fn first_cover(
    client: &reqwest::Client,
    session: &Arc<Session>,
    id: &str,
) -> Option<String> {
    let bytes = get_bytes(client, session, &format!("/playlist/v2/playlist/{id}"))
        .await
        .ok()?;
    let content = SelectedListContent::parse_from_bytes(&bytes).ok()?;
    let uri = content
        .contents
        .into_option()?
        .items
        .iter()
        .find(|item| item.uri().starts_with("spotify:track:"))?
        .uri()
        .to_owned();

    track_batch(client, session, std::slice::from_ref(&uri))
        .await
        .ok()?
        .into_iter()
        .next()?
        .cover
}

pub(super) async fn profile_json(
    client: &reqwest::Client,
    session: &Arc<Session>,
    path: &str,
) -> Result<serde_json::Value, String> {
    let response = authorized(session, "spotify profile", |access| {
        client
            .get(format!("{SPCLIENT}{path}"))
            .header(reqwest::header::AUTHORIZATION, format!("Bearer {access}"))
    })
    .await?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("spotify profile answered {status}"));
    }

    response
        .json()
        .await
        .map_err(|error| format!("spotify profile: {error}"))
}

pub(super) fn escape(value: &str) -> String {
    url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

pub(super) fn cover_url(value: &str) -> Option<String> {
    if value.is_empty() {
        return None;
    }

    if value.starts_with("https://") || value.starts_with("http://") {
        return Some(value.to_owned());
    }

    let rest = value
        .strip_prefix("spotify:image:")
        .or_else(|| value.strip_prefix("spotify:mosaic:"))?;
    let id = rest.split(':').next().filter(|id| !id.is_empty())?;
    Some(format!("{IMAGE_HOST}{id}"))
}

pub async fn following(
    client: &reqwest::Client,
    session: &Arc<Session>,
    username: &str,
) -> Result<Vec<Follow>, String> {
    if username.is_empty() {
        return Err("Sign in to Spotify again".to_owned());
    }

    let value = profile_json(
        client,
        session,
        &format!(
            "/user-profile-view/v3/profile/{}/following",
            escape(username)
        ),
    )
    .await?;

    let mut follows = Vec::new();
    for profile in value
        .get("profiles")
        .and_then(|profiles| profiles.as_array())
        .map(Vec::as_slice)
        .unwrap_or_default()
    {
        let uri = profile
            .get("uri")
            .and_then(|uri| uri.as_str())
            .unwrap_or_default();
        let (kind, id) = if let Some(id) = uri.strip_prefix("spotify:artist:") {
            (FollowKind::Artist, id)
        } else if let Some(id) = uri.strip_prefix("spotify:user:") {
            (FollowKind::User, id)
        } else {
            continue;
        };

        if id.is_empty() {
            continue;
        }

        follows.push(Follow {
            id: id.to_owned(),
            name: profile
                .get("name")
                .and_then(|name| name.as_str())
                .unwrap_or_default()
                .to_owned(),
            image: profile
                .get("image_url")
                .and_then(|image| image.as_str())
                .and_then(cover_url),
            kind,
            source: Some(Source::Spotify),
        });
    }

    Ok(follows)
}

pub async fn friends(
    client: &reqwest::Client,
    session: &Arc<Session>,
) -> Result<Vec<String>, String> {
    let value = profile_json(client, session, "/presence-view/v1/buddylist").await?;
    let mut seen = std::collections::HashSet::new();
    let mut friends = Vec::new();
    for friend in value
        .get("friends")
        .and_then(|friends| friends.as_array())
        .map(Vec::as_slice)
        .unwrap_or_default()
    {
        let user = friend.get("user");
        let uri = user
            .and_then(|user| user.get("uri"))
            .and_then(|uri| uri.as_str())
            .unwrap_or_default();
        let name = user
            .and_then(|user| user.get("name"))
            .and_then(|name| name.as_str())
            .unwrap_or_default();
        let handle = uri.strip_prefix("spotify:user:").unwrap_or(name).trim();
        if handle.is_empty() || !seen.insert(handle.to_lowercase()) {
            continue;
        }

        friends.push(handle.to_owned());
    }

    Ok(friends)
}

pub(super) async fn profile(
    client: &reqwest::Client,
    session: &Arc<Session>,
    username: &str,
) -> Result<(String, Option<String>), String> {
    let value = profile_json(
        client,
        session,
        &format!("/user-profile-view/v3/profile/{}", escape(username)),
    )
    .await?;
    let name = value
        .get("name")
        .and_then(|name| name.as_str())
        .filter(|name| !name.is_empty())
        .unwrap_or(username)
        .to_owned();
    let image = value
        .get("image_url")
        .and_then(|image| image.as_str())
        .and_then(cover_url)
        .or_else(|| {
            value
                .get("images")
                .and_then(|images| images.as_array())
                .and_then(|images| images.first())
                .and_then(|image| image.get("url"))
                .and_then(|url| url.as_str())
                .and_then(cover_url)
        });
    Ok((name, image))
}

pub(super) async fn public_playlists(
    client: &reqwest::Client,
    session: &Arc<Session>,
    username: &str,
) -> Result<Vec<Playlist>, String> {
    let value = profile_json(
        client,
        session,
        &format!(
            "/user-profile-view/v3/profile/{}/playlists",
            escape(username)
        ),
    )
    .await?;
    let playlists = value
        .get("public_playlists")
        .and_then(|playlists| playlists.as_array())
        .map(|playlists| {
            playlists
                .iter()
                .filter_map(|playlist| {
                    let uri = playlist.get("uri").and_then(|uri| uri.as_str())?;
                    let (_, id) = uri.rsplit_once(":playlist:")?;
                    Some(Playlist {
                        id: format!("spotify:playlist:{id}"),
                        name: playlist
                            .get("name")
                            .and_then(|name| name.as_str())
                            .unwrap_or_default()
                            .to_owned(),
                        subtitle: playlist
                            .get("owner_name")
                            .and_then(|owner| owner.as_str())
                            .map(str::to_owned),
                        owner: playlist
                            .get("owner_name")
                            .and_then(|owner| owner.as_str())
                            .map(str::to_owned),
                        cover: playlist
                            .get("image_url")
                            .and_then(|image| image.as_str())
                            .and_then(cover_url),
                        source: Source::Spotify,
                        ..Default::default()
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Ok(playlists)
}

pub async fn user(
    client: &reqwest::Client,
    session: &Arc<Session>,
    username: &str,
) -> Result<UserView, String> {
    let (profile, playlists) = tokio::join!(profile(client, session, username), async {
        if username.eq_ignore_ascii_case(&session.username())
            && let Ok(playlists) = library(client, session).await
        {
            return Ok(playlists);
        }

        public_playlists(client, session, username).await
    });
    let (name, image) = profile.unwrap_or_else(|_| (username.to_owned(), None));
    Ok(UserView {
        username: username.to_owned(),
        name,
        image,
        playlists: playlists?,
    })
}
