//! The JSON Jota writes into its cache, so both apps store the same contract.
//!
//! Jondo's own models carry more than the cache holds (a song's playlist, a
//! playlist's mosaic, a stream's user agent). These are the shapes a file
//! holds, and the conversions back fill in what the source can say again: the
//! id comes from the cache key, the agent from the client that resolved the
//! stream, the mosaic from the songs.

use serde::{Deserialize, Serialize};

use crate::shared::music::domain as music;

fn is_zero(value: &u32) -> bool {
    *value == 0
}

fn is_false(value: &bool) -> bool {
    !*value
}

/// Where a song is shared from.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Share {
    pub id: String,
    pub url: String,
}

/// The album a song belongs to.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Album {
    pub id: String,
    #[serde(rename = "title")]
    pub name: String,
    pub url: String,
    pub covers: Vec<String>,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub year: u32,
}

/// One of the artists credited on a song.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Artist {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub source: String,
}

/// A track, as Jota caches it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Song {
    pub id: String,
    pub url: String,
    pub name: String,
    pub duration: u32,
    pub share: Share,
    pub album: Album,
    pub artists: Vec<Artist>,
    #[serde(
        rename = "youtubeId",
        default,
        skip_serializing_if = "String::is_empty"
    )]
    pub video_id: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub broken: bool,
}

/// A playlist, as Jota caches it. The id is the cache key, not a field.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Playlist {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub cover: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub owner: String,
    #[serde(rename = "ownerId", default, skip_serializing_if = "String::is_empty")]
    pub owner_id: String,
    pub songs: Vec<Song>,
}

/// A playlist and the Spotify revision it was fetched at, so a refresh can tell
/// whether the playlist changed. Jondo does not track the revision yet, so it
/// writes the empty one, which Jota reads as "nothing to compare".
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Revisioned {
    pub revision: String,
    pub playlist: Playlist,
}

/// A playlist of a library, without its songs.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Summary {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub cover: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub covers: Vec<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub subtitle: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub owner: String,
}

/// A signed stream URL, as Jota caches it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Audio {
    pub url: String,
    pub duration: u32,
    #[serde(rename = "expireAt")]
    pub expire_at: i64,
    #[serde(rename = "videoId")]
    pub video_id: String,
    #[serde(
        rename = "clientName",
        default,
        skip_serializing_if = "String::is_empty"
    )]
    pub client_name: String,
}

/// A public profile, as Jota describes it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    pub username: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "imageUrl", default, skip_serializing_if = "String::is_empty")]
    pub image_url: String,
}

/// A YouTube channel, as Jota describes it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Channel {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub avatar: String,
}

/// One follow, as Jota caches it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Follow {
    pub id: String,
    pub name: String,
    #[serde(rename = "imageUrl", default, skip_serializing_if = "String::is_empty")]
    pub image_url: String,
    pub kind: String,
}

/// An artist and the tracks that head the page.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ArtistInfo {
    pub name: String,
    pub uri: String,
    #[serde(rename = "imageUrl", default, skip_serializing_if = "String::is_empty")]
    pub image_url: String,
    pub tracks: Vec<Song>,
}

/// An album without its songs, as the discography lists it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AlbumSummary {
    pub id: String,
    pub name: String,
    pub year: u32,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub cover: String,
    pub group: String,
}

/// Everything an artist released, in one file.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Discography {
    pub name: String,
    pub uri: String,
    pub albums: Vec<AlbumSummary>,
}

impl From<&music::ArtistView> for ArtistInfo {
    fn from(artist: &music::ArtistView) -> Self {
        Self {
            name: artist.name.clone(),
            uri: artist.id.clone(),
            image_url: artist.image.clone().unwrap_or_default(),
            tracks: artist.tracks.iter().map(Song::from).collect(),
        }
    }
}

impl ArtistInfo {
    pub fn into_domain(self, id: &str, albums: Vec<music::Album>) -> music::ArtistView {
        music::ArtistView {
            id: id.to_owned(),
            name: self.name,
            image: (!self.image_url.is_empty()).then_some(self.image_url),
            followers: 0,
            genres: Vec::new(),
            tracks: self.tracks.into_iter().map(Into::into).collect(),
            albums,
        }
    }
}

impl From<&music::Album> for AlbumSummary {
    fn from(album: &music::Album) -> Self {
        Self {
            id: album.id.clone(),
            name: album.name.clone(),
            year: album.year,
            cover: album.cover.clone().unwrap_or_default(),
            group: album.group.clone(),
        }
    }
}

impl AlbumSummary {
    pub fn into_domain(self) -> music::Album {
        music::Album {
            id: self.id,
            name: self.name,
            artists: Vec::new(),
            cover: (!self.cover.is_empty()).then_some(self.cover),
            year: self.year,
            group: self.group,
            tracks: Vec::new(),
        }
    }
}

impl From<&music::Follow> for Follow {
    fn from(follow: &music::Follow) -> Self {
        Self {
            id: follow.id.clone(),
            name: follow.name.clone(),
            image_url: follow.image.clone().unwrap_or_default(),
            kind: match follow.kind {
                music::FollowKind::Artist => "artist",
                music::FollowKind::User => "user",
                music::FollowKind::Channel => "channel",
            }
            .to_owned(),
        }
    }
}

impl From<Follow> for music::Follow {
    fn from(follow: Follow) -> Self {
        Self {
            id: follow.id,
            name: follow.name,
            image: (!follow.image_url.is_empty()).then_some(follow.image_url),
            kind: match follow.kind.as_str() {
                "user" => music::FollowKind::User,
                "channel" => music::FollowKind::Channel,
                _ => music::FollowKind::Artist,
            },
            source: Some(music::Source::Spotify),
        }
    }
}

/// The four covers a playlist row paints, taken from its first songs.
fn mosaic(songs: &[music::Song]) -> Vec<String> {
    let mut covers = Vec::with_capacity(4);
    for cover in songs.iter().filter_map(|song| song.cover.as_deref()) {
        if covers.iter().any(|seen| seen == cover) {
            continue;
        }

        covers.push(cover.to_owned());
        if covers.len() == 4 {
            break;
        }
    }

    covers
}

impl From<&music::Song> for Song {
    fn from(song: &music::Song) -> Self {
        let spotify = song.source == music::Source::Spotify;
        let share = if spotify {
            let id = music::id::bare(&song.id).to_owned();
            Share {
                url: format!("https://open.spotify.com/track/{id}"),
                id,
            }
        } else {
            Share::default()
        };

        let album_id = song.album_id.clone().unwrap_or_default();
        let album = Album {
            url: if spotify {
                format!("spotify:album:{}", music::id::bare(&album_id))
            } else {
                String::new()
            },
            id: album_id,
            name: song.album.clone(),
            covers: song.cover.iter().cloned().collect(),
            year: song.album_year,
        };

        let artists = (0..song.artists.len().max(song.artist_ids.len()))
            .map(|index| Artist {
                id: song.artist_ids.get(index).cloned().unwrap_or_default(),
                name: song.artists.get(index).cloned().unwrap_or_default(),
                source: String::new(),
            })
            .collect();

        Self {
            id: song.id.clone(),
            url: song.uri.clone().unwrap_or_default(),
            name: song.name.clone(),
            duration: song.duration,
            share,
            album,
            artists,
            video_id: song
                .video_id
                .as_deref()
                .map(music::id::bare)
                .unwrap_or_default()
                .to_owned(),
            broken: song.broken,
        }
    }
}

impl From<Song> for music::Song {
    fn from(song: Song) -> Self {
        let album_id = if song.album.id.is_empty() {
            (!song.album.url.is_empty()).then(|| song.album.url.clone())
        } else {
            Some(song.album.id.clone())
        };

        Self {
            source: match music::id::platform(&song.id) {
                music::id::Platform::Spotify => music::Source::Spotify,
                music::id::Platform::YouTube => music::Source::YouTube,
                music::id::Platform::Local => music::Source::Local,
            },
            id: song.id.clone(),
            name: song.name,
            artists: song
                .artists
                .iter()
                .map(|artist| artist.name.clone())
                .collect(),
            artist_ids: song
                .artists
                .iter()
                .map(|artist| artist.id.clone())
                .collect(),
            album: song.album.name,
            album_id,
            album_year: song.album.year,
            playlist_id: None,
            cover: song.album.covers.first().cloned(),
            duration: song.duration,
            video_id: (!song.video_id.is_empty()).then(|| music::id::video(&song.video_id)),
            uri: (!song.url.is_empty()).then_some(song.url).or(Some(song.id)),
            broken: song.broken,
        }
    }
}

impl From<&music::Playlist> for Playlist {
    fn from(playlist: &music::Playlist) -> Self {
        Self {
            name: playlist.name.clone(),
            cover: playlist.cover.clone().unwrap_or_default(),
            owner: playlist.owner.clone().unwrap_or_default(),
            owner_id: playlist.owner_id.clone().unwrap_or_default(),
            songs: playlist.songs.iter().map(Song::from).collect(),
        }
    }
}

impl Playlist {
    /// Rebuilds the playlist a file holds. The id comes from the cache key and
    /// the mosaic from the songs, so the row paints what it painted before.
    pub fn into_domain(self, id: &str) -> music::Playlist {
        let songs: Vec<music::Song> = self.songs.into_iter().map(Into::into).collect();
        let source = match music::id::platform(id) {
            music::id::Platform::Spotify => music::Source::Spotify,
            music::id::Platform::YouTube => music::Source::YouTube,
            music::id::Platform::Local => music::Source::Local,
        };

        music::Playlist {
            id: id.to_owned(),
            name: self.name,
            subtitle: (!self.owner.is_empty()).then(|| self.owner.clone()),
            cover: (!self.cover.is_empty()).then_some(self.cover),
            covers: mosaic(&songs),
            source,
            owner: (!self.owner.is_empty()).then_some(self.owner),
            owner_id: (!self.owner_id.is_empty()).then_some(self.owner_id),
            track_count: songs.len() as u32,
            songs,
            loading: false,
        }
    }
}

impl From<&music::Playlist> for Summary {
    fn from(playlist: &music::Playlist) -> Self {
        Self {
            id: playlist.id.clone(),
            name: playlist.name.clone(),
            cover: playlist.cover.clone().unwrap_or_default(),
            covers: playlist.covers.clone(),
            subtitle: playlist.subtitle.clone().unwrap_or_default(),
            owner: playlist.owner.clone().unwrap_or_default(),
        }
    }
}

impl Summary {
    pub fn into_domain(self) -> music::Playlist {
        music::Playlist {
            id: self.id,
            name: self.name,
            subtitle: (!self.subtitle.is_empty()).then_some(self.subtitle),
            cover: (!self.cover.is_empty()).then_some(self.cover),
            covers: self.covers,
            source: music::Source::Spotify,
            owner: (!self.owner.is_empty()).then_some(self.owner),
            owner_id: None,
            songs: Vec::new(),
            track_count: 0,
            loading: false,
        }
    }
}

#[cfg(test)]
#[path = "wire.test.rs"]
mod tests;
