use std::sync::Arc;

use librespot_protocol::extended_metadata::{
    BatchedEntityRequest, BatchedExtensionResponse, EntityRequest, ExtensionQuery,
};

use librespot_protocol::extension_kind::ExtensionKind;
use librespot_protocol::metadata::Track as TrackMessage;
use protobuf::{EnumOrUnknown, Message};

use crate::shared::music::domain::{Song, Source};

use super::session::*;
use super::*;

type Part = (Vec<String>, Result<Vec<Song>, String>);

pub(super) fn track_id(query: &str) -> Option<String> {
    let id = query
        .strip_prefix("spotify:track:")
        .unwrap_or(query)
        .split(['?', '#'])
        .next()?
        .trim();
    (matches!(id.len(), 21..=22) && id.bytes().all(|byte| byte.is_ascii_alphanumeric()))
        .then(|| id.to_owned())
}

pub(super) async fn get_bytes(
    client: &reqwest::Client,
    session: &Arc<Session>,
    path: &str,
) -> Result<Vec<u8>, String> {
    let response = authorized(session, "spotify", |access| {
        client
            .get(format!("{SPCLIENT}{path}"))
            .header(reqwest::header::AUTHORIZATION, format!("Bearer {access}"))
            .header(reqwest::header::ACCEPT, "application/octet-stream")
    })
    .await?;

    let status = response.status();
    let bytes = response
        .bytes()
        .await
        .map_err(|error| format!("spotify: {error}"))?;

    if !status.is_success() {
        let body = String::from_utf8_lossy(&bytes);
        let detail = body.chars().take(200).collect::<String>();
        return Err(format!("spotify answered {status}: {detail}"));
    }

    Ok(bytes.to_vec())
}

const TRACKS_BATCH: usize = 50;
const TRACKS_PARALLEL: usize = 8;

pub(super) async fn tracks(
    client: &reqwest::Client,
    session: &Arc<Session>,
    uris: &[String],
) -> Vec<Song> {
    let unique = dedupe(uris);
    let chunks: Vec<&[String]> = unique.chunks(TRACKS_BATCH).collect();
    let permits = std::sync::Arc::new(tokio::sync::Semaphore::new(TRACKS_PARALLEL));
    let mut set = tokio::task::JoinSet::new();

    for (index, chunk) in chunks.iter().enumerate() {
        let client = client.clone();
        let session = session.clone();
        let chunk = chunk.to_vec();
        let permits = permits.clone();
        set.spawn(async move {
            let permit = permits.acquire_owned().await.ok();
            let result = track_batch(&client, &session, &chunk).await;
            (index, chunk, result, permit)
        });
    }

    let mut parts: Vec<Option<Part>> = chunks.iter().map(|_| None).collect();
    while let Some(joined) = set.join_next().await {
        if let Ok((index, chunk, result, _permit)) = joined {
            parts[index] = Some((chunk, result));
        }
    }

    let mut found = std::collections::HashMap::with_capacity(unique.len());
    for part in parts {
        match part {
            Some((chunk, Ok(songs))) => found.extend(chunk.into_iter().zip(songs)),
            Some((chunk, Err(error))) => {
                log::warn!("spotify tracks: {error}");
                found.extend(chunk.into_iter().map(|uri| {
                    let song = broken_song(&uri);
                    (uri, song)
                }));
            }
            None => {}
        }
    }

    reuse(uris, &found)
}

pub(super) fn dedupe(uris: &[String]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    uris.iter()
        .filter(|uri| seen.insert(uri.as_str()))
        .cloned()
        .collect()
}

pub(super) fn reuse(uris: &[String], found: &std::collections::HashMap<String, Song>) -> Vec<Song> {
    uris.iter()
        .map(|uri| {
            found
                .get(uri.as_str())
                .cloned()
                .unwrap_or_else(|| broken_song(uri))
        })
        .collect()
}

pub(super) async fn track_batch(
    client: &reqwest::Client,
    session: &Arc<Session>,
    uris: &[String],
) -> Result<Vec<Song>, String> {
    let request = BatchedEntityRequest {
        entity_request: uris
            .iter()
            .map(|uri| EntityRequest {
                entity_uri: uri.clone(),
                query: vec![ExtensionQuery {
                    extension_kind: EnumOrUnknown::new(ExtensionKind::TRACK_V4),
                    ..Default::default()
                }],
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    };

    let body = request
        .write_to_bytes()
        .map_err(|error| format!("spotify tracks: {error}"))?;

    let response = authorized(session, "spotify tracks", |access| {
        client
            .post(format!("{SPCLIENT}/extended-metadata/v0/extended-metadata"))
            .header(reqwest::header::AUTHORIZATION, format!("Bearer {access}"))
            .header(reqwest::header::CONTENT_TYPE, "application/x-protobuf")
            .body(body.clone())
    })
    .await?;

    let status = response.status();
    let bytes = response
        .bytes()
        .await
        .map_err(|error| format!("spotify tracks: {error}"))?;
    if !status.is_success() {
        return Err(format!("spotify tracks answered {status}"));
    }

    let parsed = BatchedExtensionResponse::parse_from_bytes(&bytes)
        .map_err(|error| format!("spotify tracks: {error}"))?;

    let mut by_uri = std::collections::HashMap::new();
    for array in &parsed.extended_metadata {
        for entry in &array.extension_data {
            if matches!(
                entry.header.as_ref().map(|header| header.status_code),
                Some(0 | 200)
            ) {
                by_uri.insert(entry.entity_uri.as_str(), entry);
            }
        }
    }

    let mut songs = Vec::with_capacity(uris.len());
    for uri in uris {
        let Some(entry) = by_uri.get(uri.as_str()) else {
            songs.push(broken_song(uri));
            continue;
        };

        let Some(data) = entry.extension_data.as_ref() else {
            songs.push(broken_song(uri));
            continue;
        };

        match TrackMessage::parse_from_bytes(&data.value) {
            Ok(track) => songs.push(song_from_track(uri, &track)),
            Err(_) => songs.push(broken_song(uri)),
        }
    }

    Ok(songs)
}

pub(super) fn song_from_track(uri: &str, track: &TrackMessage) -> Song {
    let id = uri.strip_prefix("spotify:track:").unwrap_or(uri);
    let album = track.album.get_or_default();
    let images = if album.cover_group.image.is_empty() {
        &album.cover
    } else {
        &album.cover_group.image
    };

    let cover = images
        .iter()
        .max_by_key(|image| image.width)
        .and_then(|image| image.file_id.as_deref().and_then(file_cover));
    let artists = track
        .artist
        .iter()
        .filter_map(|artist| artist.name.as_deref().and_then(non_empty))
        .collect();

    let album_id = album.gid.as_deref().filter(|gid| !gid.is_empty()).map(hex);
    let artist_ids = track
        .artist
        .iter()
        .filter_map(|artist| artist.gid.as_deref().filter(|gid| !gid.is_empty()).map(hex))
        .collect();
    Song {
        id: format!("spotify:track:{id}"),
        name: track.name.clone().unwrap_or_default(),
        artists,
        artist_ids,
        album: album.name.clone().unwrap_or_default(),
        album_id,
        playlist_id: None,
        cover,
        duration: (track.duration.unwrap_or(0).max(0) / 1000) as u32,
        source: Source::Spotify,
        video_id: None,
        uri: Some(uri.to_owned()),
        broken: false,
    }
}

pub(super) fn broken_song(uri: &str) -> Song {
    let id = uri.strip_prefix("spotify:track:").unwrap_or(uri);
    Song {
        id: format!("spotify:track:{id}"),
        name: "Unavailable track".to_owned(),
        source: Source::Spotify,
        uri: Some(uri.to_owned()),
        broken: true,
        ..Default::default()
    }
}

pub(super) fn picture(bytes: &[u8]) -> Option<String> {
    (!bytes.is_empty()).then(|| format!("{IMAGE_HOST}{}", hex(bytes)))
}

pub(super) fn file_cover(bytes: &[u8]) -> Option<String> {
    (!bytes.is_empty()).then(|| format!("{IMAGE_HOST}{}", hex(bytes)))
}

pub(super) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub(super) fn non_empty(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_owned())
}
