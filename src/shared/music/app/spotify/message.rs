use std::sync::Arc;

use librespot_protocol::metadata::album::Type as AlbumType;
use librespot_protocol::metadata::{
    Album as AlbumMessage, Artist as ArtistMessage, Track as TrackMessage,
};

use protobuf::Message;

use crate::shared::music::domain::{Album, ArtistRef, ArtistView, Song, Source};

use super::{playlist::*, session::*, tracks::*};

pub(super) fn gid(value: Option<&Vec<u8>>) -> Option<String> {
    value.filter(|gid| !gid.is_empty()).map(|gid| hex(gid))
}

pub(super) fn images(
    primary: &[librespot_protocol::metadata::Image],
    fallback: &[librespot_protocol::metadata::Image],
) -> Option<String> {
    let list = if primary.is_empty() {
        fallback
    } else {
        primary
    };

    list.iter()
        .max_by_key(|image| image.width)
        .and_then(|image| image.file_id.as_deref().and_then(file_cover))
}

pub(super) fn album_cover(album: &AlbumMessage) -> Option<String> {
    images(&album.cover_group.image, &album.cover)
}

pub(super) fn artist_image(artist: &ArtistMessage) -> Option<String> {
    images(&artist.portrait_group.image, &artist.portrait)
}

pub(super) fn album_artists(album: &AlbumMessage) -> Vec<ArtistRef> {
    album
        .artist
        .iter()
        .filter_map(|artist| {
            Some(ArtistRef {
                id: gid(artist.gid.as_ref())?,
                name: artist.name.clone().unwrap_or_default(),
            })
        })
        .collect()
}

pub(super) fn album_group(album: &AlbumMessage) -> String {
    match album.type_.and_then(|kind| kind.enum_value().ok()) {
        Some(AlbumType::SINGLE) => "single",
        Some(AlbumType::COMPILATION) => "compilation",
        _ => "album",
    }
    .to_owned()
}

pub(super) fn album_year(album: &AlbumMessage) -> u32 {
    album
        .date
        .as_ref()
        .map(|date| date.year().max(0) as u32)
        .unwrap_or_default()
}

pub(super) fn meta_song(
    track: &TrackMessage,
    album: Option<&AlbumMessage>,
    album_id: &str,
) -> Song {
    let hex_id = gid(track.gid.as_ref()).unwrap_or_default();
    let id = if hex_id.is_empty() {
        String::new()
    } else {
        hex_to_base62(&hex_id)
    };

    let fallback_album = track.album.as_ref();
    let source_album = album.or(fallback_album);
    let album_name = source_album
        .and_then(|album| album.name.clone())
        .unwrap_or_default();
    let cover = source_album.and_then(album_cover);
    let artists: Vec<ArtistRef> = track
        .artist
        .iter()
        .filter_map(|artist| {
            Some(ArtistRef {
                id: gid(artist.gid.as_ref())?,
                name: artist.name.clone().unwrap_or_default(),
            })
        })
        .collect();
    Song {
        id: format!("spotify:track:{id}"),
        name: track.name.clone().unwrap_or_default(),
        artists: artists.iter().map(|artist| artist.name.clone()).collect(),
        artist_ids: artists.iter().map(|artist| artist.id.clone()).collect(),
        album: album_name,
        album_id: Some(album_id.to_owned()),
        playlist_id: None,
        cover,
        duration: (track.duration.unwrap_or(0).max(0) / 1000) as u32,
        source: Source::Spotify,
        video_id: None,
        uri: (!id.is_empty()).then(|| format!("spotify:track:{id}")),
        broken: false,
    }
}

pub(super) fn album_from_message(id: &str, message: &AlbumMessage) -> Album {
    let name = message.name.clone().unwrap_or_default();
    let artists = album_artists(message);
    let cover = album_cover(message);
    let tracks = message
        .disc
        .iter()
        .flat_map(|disc| disc.track.iter())
        .map(|track| meta_song(track, Some(message), id))
        .collect();
    Album {
        id: id.to_owned(),
        name,
        artists,
        cover,
        year: album_year(message),
        group: album_group(message),
        tracks,
    }
}

pub(super) fn album_summary(message: &AlbumMessage, group: &str) -> Option<Album> {
    let id = gid(message.gid.as_ref())?;
    Some(Album {
        id,
        name: message.name.clone().unwrap_or_default(),
        artists: album_artists(message),
        cover: album_cover(message),
        year: album_year(message),
        group: group.to_owned(),
        tracks: Vec::new(),
    })
}

pub(super) fn artist_from_message(id: &str, message: &ArtistMessage) -> ArtistView {
    let mut tracks = Vec::new();
    for group in &message.top_track {
        for track in &group.track {
            let album = track.album.as_ref();
            let album_id = album
                .and_then(|album| gid(album.gid.as_ref()))
                .unwrap_or_default();
            tracks.push(meta_song(track, album, &album_id));
        }
    }

    let mut seen = std::collections::HashSet::new();
    tracks.retain(|song| seen.insert(song.id.clone()));

    let mut albums = Vec::new();
    let groups = [
        ("album", &message.album_group),
        ("single", &message.single_group),
        ("compilation", &message.compilation_group),
        ("appears_on", &message.appears_on_group),
    ];
    let mut seen = std::collections::HashSet::new();
    for (group, list) in groups {
        for entry in list {
            for album in &entry.album {
                if let Some(album) = album_summary(album, group)
                    && seen.insert(album.id.clone())
                {
                    albums.push(album);
                }
            }
        }
    }

    ArtistView {
        id: id.to_owned(),
        name: message.name.clone().unwrap_or_default(),
        image: artist_image(message),
        followers: 0,
        genres: Vec::new(),
        tracks,
        albums,
    }
}

pub async fn enrich_albums(client: &reqwest::Client, session: &Arc<Session>, albums: &mut [Album]) {
    let targets: Vec<(usize, String)> = albums
        .iter()
        .enumerate()
        .filter(|(_, album)| album.name.is_empty() || album.cover.is_none())
        .map(|(index, album)| (index, album.id.clone()))
        .collect();

    let mut set = tokio::task::JoinSet::new();
    for (index, id) in targets {
        let http = client.clone();
        let session = session.clone();
        set.spawn(async move {
            let bytes = metadata(&http, &session, "album", &id).await.ok()?;
            let message = AlbumMessage::parse_from_bytes(&bytes).ok()?;
            let name = message.name.clone().unwrap_or_default();
            let year = album_year(&message);
            let cover = album_cover(&message);
            let artists = album_artists(&message);
            Some((index, name, year, cover, artists))
        });
        if set.len() >= 6
            && let Some(Ok(Some((index, name, year, cover, artists)))) = set.join_next().await
        {
            apply_album(albums, index, name, year, cover, artists);
        }
    }

    while let Some(result) = set.join_next().await {
        if let Ok(Some((index, name, year, cover, artists))) = result {
            apply_album(albums, index, name, year, cover, artists);
        }
    }
}

pub(super) fn apply_album(
    albums: &mut [Album],
    index: usize,
    name: String,
    year: u32,
    cover: Option<String>,
    artists: Vec<ArtistRef>,
) {
    let Some(album) = albums.get_mut(index) else {
        return;
    };

    if album.name.is_empty() {
        album.name = name;
    }

    if album.year == 0 {
        album.year = year;
    }

    if album.cover.is_none() {
        album.cover = cover;
    }

    if album.artists.is_empty() {
        album.artists = artists;
    }
}
