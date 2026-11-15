use std::sync::Arc;

use librespot_protocol::metadata::{Album as AlbumMessage, Artist as ArtistMessage};
use protobuf::Message;

use crate::shared::music::domain::{Album, ArtistRef, ArtistView};
use crate::shared::storage::cache::{self, wire};

use super::*;
use super::{message::*, playlist::*, session::*, tracks::*};

pub async fn album(
    client: &reqwest::Client,
    session: &Arc<Session>,
    id: &str,
) -> Result<Album, String> {
    let bytes = metadata(client, session, "album", id).await?;
    let message = AlbumMessage::parse_from_bytes(&bytes)
        .map_err(|error| format!("spotify album: {error}"))?;
    let mut album = album_from_message(id, &message);
    let uris: Vec<String> = album
        .tracks
        .iter()
        .filter_map(|song| song.uri.clone())
        .collect();
    if !uris.is_empty() {
        let full = tracks(client, session, &uris).await;
        if full.iter().any(|song| !song.name.is_empty()) {
            album.tracks = full;
        }
    }

    let songs: Vec<wire::Song> = album
        .tracks
        .iter()
        .map(|song| {
            let mut song = wire::Song::from(song);
            song.album.year = album.year;
            song.album.covers = album.cover.iter().cloned().collect();
            song
        })
        .collect();
    if let Err(error) = cache::put(&format!("spotify/albums/{id}"), &songs, None) {
        log::debug!("spotify album: {error}");
    }

    Ok(album)
}

/// The album a file holds, and when it was saved. Jota keeps its songs, so the
/// header comes from the album the songs name.
pub fn cached_album(id: &str) -> Option<(Album, i64)> {
    let (songs, saved) = cache::get_with_meta::<Vec<wire::Song>>(&format!("spotify/albums/{id}"))?;
    Some((from_cache(id, songs), saved))
}

fn from_cache(id: &str, songs: Vec<wire::Song>) -> Album {
    let first = songs.first();
    Album {
        id: id.to_owned(),
        name: first
            .map(|song| song.album.name.clone())
            .unwrap_or_default(),
        artists: first
            .map(|song| {
                song.artists
                    .iter()
                    .map(|artist| ArtistRef {
                        id: artist.id.clone(),
                        name: artist.name.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default(),
        cover: first.and_then(|song| song.album.covers.first().cloned()),
        year: first.map(|song| song.album.year).unwrap_or_default(),
        group: String::new(),
        tracks: songs.into_iter().map(Into::into).collect(),
    }
}

pub async fn artist(
    client: &reqwest::Client,
    session: &Arc<Session>,
    id: &str,
) -> Result<ArtistView, String> {
    let info_name = format!("spotify/artists/{id}");
    let discography_name = format!("spotify/discography/{id}");

    let bytes = metadata(client, session, "artist", id).await?;
    let message = ArtistMessage::parse_from_bytes(&bytes)
        .map_err(|error| format!("spotify artist: {error}"))?;
    let mut artist = artist_from_message(id, &message);
    let uris: Vec<String> = artist
        .tracks
        .iter()
        .filter_map(|song| song.uri.clone())
        .collect();
    if !uris.is_empty() {
        let full = tracks(client, session, &uris).await;
        if full.iter().any(|song| !song.name.is_empty()) {
            artist.tracks = full;
        }
    }

    let discography = wire::Discography {
        name: artist.name.clone(),
        uri: artist.id.clone(),
        albums: artist.albums.iter().map(wire::AlbumSummary::from).collect(),
    };
    if let Err(error) = cache::put(&info_name, &wire::ArtistInfo::from(&artist), None) {
        log::debug!("spotify artist: {error}");
    }
    if let Err(error) = cache::put(&discography_name, &discography, None) {
        log::debug!("spotify discography: {error}");
    }

    Ok(artist)
}

/// The artist a file holds with its discography, and when the artist was saved.
/// Both files are needed: the page shows the tracks and the releases.
pub fn cached_artist(id: &str) -> Option<(ArtistView, i64)> {
    let (info, saved) = cache::get_with_meta::<wire::ArtistInfo>(&format!("spotify/artists/{id}"))?;
    let discography = cache::get::<wire::Discography>(&format!("spotify/discography/{id}"))?;
    let albums = discography
        .albums
        .into_iter()
        .map(wire::AlbumSummary::into_domain)
        .collect();

    Some((info.into_domain(id, albums), saved))
}

pub async fn search(
    client: &reqwest::Client,
    session: &Arc<Session>,
    query: &str,
) -> Result<crate::shared::music::domain::SpotifySearch, String> {
    let query = query.trim();
    if query.is_empty() {
        return Err("Enter something to search".to_owned());
    }

    if let Some(id) = track_id(query) {
        let uri = format!("spotify:track:{id}");
        let songs = tracks(client, session, std::slice::from_ref(&uri)).await;
        return match songs.into_iter().next() {
            Some(song) if !song.broken => {
                Ok(crate::shared::music::domain::SpotifySearch { tracks: vec![song] })
            }
            _ => Err("Spotify could not open this track".to_owned()),
        };
    }

    let encoded: String = url::form_urlencoded::byte_serialize(query.as_bytes()).collect();
    let response = authorized(session, "spotify search", |access| {
        client
            .get(format!(
                "{SPCLIENT}/context-resolve/v1/spotify:search:{encoded}"
            ))
            .header(reqwest::header::AUTHORIZATION, format!("Bearer {access}"))
    })
    .await?;
    let status = response.status();
    if !status.is_success() {
        return Err(match status.as_u16() {
            401 => "Spotify session expired. Reconnect it in Settings.".to_owned(),
            429 => "Spotify is rate limited. Try again in a moment.".to_owned(),
            _ => format!("spotify search answered {status}"),
        });
    }

    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|error| format!("spotify search: {error}"))?;
    let uris: Vec<String> = value
        .get("pages")
        .and_then(|pages| pages.as_array())
        .map(|pages| {
            pages
                .iter()
                .flat_map(|page| {
                    page.get("tracks")
                        .and_then(|tracks| tracks.as_array())
                        .map(Vec::as_slice)
                        .unwrap_or_default()
                })
                .filter_map(|track| track.get("uri").and_then(|uri| uri.as_str()))
                .filter(|uri| uri.starts_with("spotify:track:"))
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();

    let songs = tracks(client, session, &uris).await;
    Ok(crate::shared::music::domain::SpotifySearch { tracks: songs })
}
