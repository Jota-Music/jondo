//! `jota.backup.json`, the document a Jota library travels in.
//!
//! Jota keeps every device converging through one JSON file in a folder the
//! user syncs with whatever they already use. The shape is Jota's: the sections
//! are read and written here exactly as it writes them, so the same folder can
//! be shared by both apps.

use std::collections::BTreeMap;

use crate::shared::music::domain::{Song, Source};
use crate::shared::storage::{LocalPlaylist, Store};

use super::{document::*, format::*, sync::*};

pub(super) fn playlists(
    document: &mut Document,
    store: &mut Store,
    mode: Mode,
    apply: bool,
) -> usize {
    let remote = document.playlists.take().unwrap_or_default();

    if apply {
        for (id, song) in &remote.songs {
            let video = song.youtube_id.trim();
            if video.is_empty() || store.youtube_picks.contains(id) {
                continue;
            }

            store.pick(id, video);
        }
    }

    let mut graves = store.playlist_graves.clone();
    for (id, at) in &remote.deleted {
        let stored = stored_id(id.trim());
        if *at > *graves.get(&stored).unwrap_or(&0) {
            graves.insert(stored, *at);
        }
    }

    let mut external = Vec::new();
    let mut incoming = Vec::new();
    for item in remote.items {
        if item.source.as_deref() == Some("local") {
            incoming.push(item);
        } else {
            external.push(item);
        }
    }

    let mut local_refs = Refs::default();
    let mut merged: BTreeMap<String, WirePlaylist> = BTreeMap::new();
    for playlist in &store.playlists {
        let entry = local_refs.playlist(playlist);
        merged.insert(entry.id.clone(), entry);
    }

    let mut metadata = Refs {
        songs: remote.songs,
        albums: remote.albums,
        artists: remote.artists,
    };

    metadata.absorb(local_refs);

    for mut item in incoming {
        let id = stored_id(item.id.trim());
        if id.is_empty() {
            continue;
        }

        item.id = id.clone();
        item.source = Some("local".to_owned());
        item.name = item.name.trim().to_owned();
        if item.name.is_empty() {
            item.name = DEFAULT_NAME.to_owned();
        }

        item.songs
            .retain(|reference| !reference.id.trim().is_empty());

        if let Some(at) = graves.get(&id).copied() {
            if mode == Mode::Newer && item.modified_at <= at {
                continue;
            }

            graves.remove(&id);
        }

        match merged.get_mut(&id) {
            Some(local)
                if !wins(
                    item.modified_at,
                    local.modified_at,
                    item.songs.len(),
                    local.songs.len(),
                ) => {}
            _ => {
                merged.insert(id, item);
            }
        }
    }

    let mut items: Vec<WirePlaylist> = merged.into_values().collect();
    items.sort_by(|left, right| left.id.cmp(&right.id));
    items.extend(external);

    let mut section = Playlists {
        items,
        deleted: graves.clone(),
        ..Default::default()
    };

    for item in &section.items {
        for reference in &item.songs {
            let Some(song) = metadata.songs.get(&reference.id) else {
                continue;
            };

            section
                .songs
                .entry(reference.id.clone())
                .or_insert_with(|| song.clone());
            if let Some(album) = &song.album
                && !album.id.is_empty()
            {
                let known = metadata.albums.get(&album.id).unwrap_or(&album.album);
                section
                    .albums
                    .entry(album.id.clone())
                    .or_insert_with(|| known.clone());
            }

            for artist in &song.artists {
                if artist.id.is_empty() {
                    continue;
                }

                let known = metadata.artists.get(&artist.id).unwrap_or(&artist.artist);
                section
                    .artists
                    .entry(artist.id.clone())
                    .or_insert_with(|| known.clone());
            }
        }
    }

    for id in &store.youtube_picks {
        let Some(video) = store.youtube_links.get(id) else {
            continue;
        };

        section.songs.entry(id.clone()).or_default().youtube_id = video.clone();
    }

    let mut changed = 0;
    if apply {
        for item in section
            .items
            .iter()
            .filter(|item| item.source.as_deref() == Some("local"))
        {
            let playlist = local_playlist(item, &section);
            match store
                .playlists
                .iter_mut()
                .find(|known| known.id == playlist.id)
            {
                Some(existing) => {
                    if wins(
                        item.modified_at,
                        milliseconds(existing.modified),
                        item.songs.len(),
                        existing.songs.len(),
                    ) {
                        existing.name = playlist.name;
                        existing.songs = playlist.songs;
                        existing.modified = playlist.modified;
                        changed += 1;
                    }
                }
                None => {
                    store.playlists.insert(0, playlist);
                    changed += 1;
                }
            }
        }

        store.playlist_graves = graves;
    }

    document.playlists = (!section.empty()).then_some(section);
    changed
}

pub(super) fn local_playlist(item: &WirePlaylist, section: &Playlists) -> LocalPlaylist {
    let songs = item
        .songs
        .iter()
        .map(|reference| {
            let song = section.songs.get(&reference.id);
            song_from_wire(&reference.id, song, section)
        })
        .collect();
    LocalPlaylist {
        id: item.id.clone(),
        name: if item.name.trim().is_empty() {
            DEFAULT_NAME.to_owned()
        } else {
            item.name.trim().to_owned()
        },
        songs,
        modified: seconds(item.modified_at),
    }
}

pub(super) fn song_from_wire(id: &str, entry: Option<&WireSong>, section: &Playlists) -> Song {
    let source = source_of(id);
    let Some(entry) = entry else {
        return Song {
            id: id.to_owned(),
            source,
            uri: (source == Source::Spotify).then(|| id.to_owned()),
            ..Default::default()
        };
    };

    let (album, album_id, cover) = match &entry.album {
        Some(reference) if reference.id.is_empty() => (
            reference.album.title.clone(),
            None,
            reference.album.covers.first().cloned(),
        ),
        Some(reference) => {
            let known = section
                .albums
                .get(&reference.id)
                .unwrap_or(&reference.album);
            (
                known.title.clone(),
                Some(album_id(&reference.id)),
                known.covers.first().cloned(),
            )
        }
        None => (String::new(), None, None),
    };

    let mut artists = Vec::with_capacity(entry.artists.len());
    let mut artist_ids = Vec::with_capacity(entry.artists.len());
    for reference in &entry.artists {
        let known = if reference.id.is_empty() {
            &reference.artist
        } else {
            section
                .artists
                .get(&reference.id)
                .unwrap_or(&reference.artist)
        };

        artists.push(known.name.clone());
        artist_ids.push(if reference.id.is_empty() {
            String::new()
        } else {
            artist_id(&reference.id)
        });
    }

    let video_id = if entry.youtube_id.is_empty() {
        video_of(id)
    } else {
        Some(entry.youtube_id.clone())
    };

    Song {
        id: id.to_owned(),
        name: entry.name.clone(),
        artists,
        album,
        album_id,
        playlist_id: None,
        artist_ids,
        cover,
        duration: u32::try_from(entry.duration.max(0)).unwrap_or_default(),
        source,
        video_id,
        uri: (source == Source::Spotify).then(|| id.to_owned()),
        broken: entry.broken,
    }
}
