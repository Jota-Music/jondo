//! `jota.backup.json`, the document a Jota library travels in.
//!
//! Jota keeps every device converging through one JSON file in a folder the
//! user syncs with whatever they already use. The shape is Jota's: the sections
//! are read and written here exactly as it writes them, so the same folder can
//! be shared by both apps.

use std::collections::BTreeMap;

use crate::shared::music::domain::{Song, Source};
use crate::shared::storage::LocalPlaylist;

use super::document::*;

pub(crate) fn zero(value: &i64) -> bool {
    *value == 0
}

pub(crate) fn is_false(value: &bool) -> bool {
    !*value
}

/// Jota derives a stable local id for a playlist whose id came from another
/// instance, so every device maps it to the same record.
pub(crate) fn stored_id(id: &str) -> String {
    if id.starts_with("local:") {
        return id.to_owned();
    }

    format!("local:{:016x}", fnv1a64(id))
}

pub(crate) fn fnv1a64(text: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }

    hash
}

pub(crate) fn wins(remote: i64, local: i64, remote_songs: usize, local_songs: usize) -> bool {
    if remote != local {
        return remote > local;
    }

    remote == 0 && local_songs == 0 && remote_songs > 0
}

pub(crate) fn seconds(at: i64) -> u64 {
    (at / 1000).max(0) as u64
}

pub(crate) fn milliseconds(seconds: u64) -> i64 {
    (seconds as i64) * 1000
}

pub(crate) fn source_name(source: Source) -> &'static str {
    match source {
        Source::Spotify => "spotify",
        Source::YouTube => "youtube",
        Source::Local => "local",
    }
}

pub(crate) fn source_of(id: &str) -> Source {
    if id.starts_with("youtube:") {
        Source::YouTube
    } else if id.starts_with("spotify:") {
        Source::Spotify
    } else {
        Source::Local
    }
}

/// The album and artist ids jondo keeps are the 32-hex gids the partner API
/// answers; the document carries the public base62 ids, so both are converted.
pub(crate) fn public_id(id: &str) -> String {
    if id.len() == 32 && id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        crate::shared::spotify::hex_to_base62(id)
    } else {
        id.to_owned()
    }
}

pub(crate) fn stored_provider_id(id: &str) -> String {
    crate::shared::spotify::id_to_hex(id)
}

pub(crate) fn album_id(id: &str) -> String {
    let bare = id.strip_prefix("spotify:album:").unwrap_or(id);
    stored_provider_id(bare)
}

pub(crate) fn artist_id(id: &str) -> String {
    let bare = id.strip_prefix("spotify:artist:").unwrap_or(id);
    stored_provider_id(bare)
}

pub(crate) fn artist_key(id: &str, source: Source) -> String {
    if id.contains(':') || source == Source::Local {
        return id.to_owned();
    }

    format!("{}:{id}", source_name(source))
}

pub(crate) fn video_of(id: &str) -> Option<String> {
    id.strip_prefix("youtube:")
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

#[derive(Default)]
pub(crate) struct Refs {
    pub(crate) songs: BTreeMap<String, WireSong>,
    pub(crate) albums: BTreeMap<String, WireAlbum>,
    pub(crate) artists: BTreeMap<String, WireArtist>,
}

impl Refs {
    pub(crate) fn playlist(&mut self, playlist: &LocalPlaylist) -> WirePlaylist {
        let mut songs = Vec::with_capacity(playlist.songs.len());
        for song in &playlist.songs {
            if song.id.is_empty() {
                continue;
            }

            self.song(song);
            songs.push(SongRef {
                id: song.id.clone(),
            });
        }

        WirePlaylist {
            id: playlist.id.clone(),
            source: Some("local".to_owned()),
            name: playlist.name.clone(),
            owner: None,
            modified_at: milliseconds(playlist.modified),
            songs,
        }
    }

    pub(crate) fn song(&mut self, song: &Song) {
        if self.songs.contains_key(&song.id) {
            return;
        }

        let covers: Vec<String> = song.cover.iter().cloned().collect();
        let album = match &song.album_id {
            Some(id) if !id.is_empty() => {
                let id = format!("spotify:album:{}", public_id(id));
                self.albums.entry(id.clone()).or_insert_with(|| WireAlbum {
                    title: song.album.clone(),
                    covers: covers.clone(),
                    year: 0,
                });
                Some(AlbumRef {
                    id,
                    album: WireAlbum::default(),
                })
            }
            _ if song.album.is_empty() && covers.is_empty() => None,
            _ => Some(AlbumRef {
                id: String::new(),
                album: WireAlbum {
                    title: song.album.clone(),
                    covers,
                    year: 0,
                },
            }),
        };

        let mut artists = Vec::with_capacity(song.artists.len());
        for (index, name) in song.artists.iter().enumerate() {
            let id = song
                .artist_ids
                .get(index)
                .filter(|id| !id.is_empty())
                .map(|id| artist_key(&public_id(id), song.source));
            match id {
                Some(id) => {
                    self.artists
                        .entry(id.clone())
                        .or_insert_with(|| WireArtist {
                            name: name.clone(),
                            source: source_name(song.source).to_owned(),
                        });
                    artists.push(ArtistRef {
                        id,
                        artist: WireArtist::default(),
                    });
                }
                None => artists.push(ArtistRef {
                    id: String::new(),
                    artist: WireArtist {
                        name: name.clone(),
                        source: source_name(song.source).to_owned(),
                    },
                }),
            }
        }

        let youtube_id = song
            .video_id
            .as_deref()
            .filter(|id| video_of(&song.id).as_deref() != Some(*id))
            .unwrap_or_default()
            .to_owned();

        self.songs.insert(
            song.id.clone(),
            WireSong {
                name: song.name.clone(),
                duration: i64::from(song.duration),
                youtube_id,
                broken: song.broken,
                album,
                artists,
            },
        );
    }

    pub(crate) fn absorb(&mut self, other: Refs) {
        for (id, song) in other.songs {
            self.songs.entry(id).or_insert(song);
        }

        for (id, album) in other.albums {
            self.albums.entry(id).or_insert(album);
        }

        for (id, artist) in other.artists {
            self.artists.entry(id).or_insert(artist);
        }
    }
}
