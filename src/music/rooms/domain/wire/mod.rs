use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::shared::music::domain::{Song, Source};

// The relay forwards frames between every Jota and Jondo member, so the wire
// shape has to stay Jota's: a Song carries an album object, artist objects and
// a flat youtubeId. Jondo's own model is flatter, so a room message is converted
// on the way in and on the way out instead of leaking one app's storage shape
// into the other's.

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct WireSong {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub duration: u32,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub share: Share,
    #[serde(default, deserialize_with = "album")]
    pub album: Album,
    #[serde(default, deserialize_with = "artists")]
    pub artists: Vec<Artist>,
    #[serde(default, rename = "youtubeId")]
    pub youtube_id: Option<String>,
    #[serde(default)]
    pub broken: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Share {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub url: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Album {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub url: String,
    #[serde(default, deserialize_with = "strings")]
    pub covers: Vec<String>,
    #[serde(default)]
    pub year: u32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Artist {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub source: String,
}

fn album<'de, D>(deserializer: D) -> Result<Album, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = Option::<Value>::deserialize(deserializer)?;
    Ok(match raw {
        Some(Value::String(title)) => Album {
            title,
            ..Default::default()
        },
        Some(Value::Object(map)) => serde_json::from_value(Value::Object(map)).unwrap_or_default(),
        _ => Album::default(),
    })
}

fn artists<'de, D>(deserializer: D) -> Result<Vec<Artist>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = Option::<Value>::deserialize(deserializer)?;
    let Some(Value::Array(items)) = raw else {
        return Ok(Vec::new());
    };

    Ok(items
        .into_iter()
        .filter_map(|item| match item {
            Value::String(name) => Some(Artist {
                name,
                ..Default::default()
            }),
            Value::Object(map) => {
                let mut artist: Artist =
                    serde_json::from_value(Value::Object(map)).unwrap_or_default();
                artist.id = artist.id.trim().to_owned();
                Some(artist)
            }
            _ => None,
        })
        .collect())
}

fn strings<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = Option::<Vec<Value>>::deserialize(deserializer)?;
    Ok(raw
        .unwrap_or_default()
        .into_iter()
        .filter_map(|item| item.as_str().map(str::to_owned))
        .collect())
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Playback {
    #[serde(default)]
    pub song_id: String,
    #[serde(default)]
    pub song: Option<WireSong>,
    #[serde(default)]
    pub youtube_id: Option<String>,
    #[serde(default)]
    pub index: usize,
    #[serde(default)]
    pub shuffle: Option<bool>,
    #[serde(default)]
    pub repeat: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub binary: Option<String>,
    #[serde(default)]
    pub playing: bool,
    #[serde(default)]
    pub position_ms: f64,
    #[serde(default)]
    pub at: Option<i64>,
    #[serde(default)]
    #[serde(rename = "gen")]
    pub generation: Option<String>,
    #[serde(default)]
    pub epoch: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Control {
    pub action: String,
    #[serde(default)]
    pub position_ms: f64,
    #[serde(default)]
    pub on: Option<bool>,
    #[serde(default)]
    pub seed: Option<u64>,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub songs: Vec<WireSong>,
    #[serde(default)]
    pub at: usize,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub youtube_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "t")]
pub enum ServerMessage {
    #[serde(rename = "role")]
    Role { role: String },
    #[serde(rename = "members")]
    Members {
        count: usize,
        #[serde(default)]
        epoch: String,
    },
    #[serde(rename = "pong")]
    Pong { id: u64, at: i64, echo: i64 },
    #[serde(rename = "play")]
    Play {
        #[serde(rename = "gen")]
        generation: String,
        #[serde(default)]
        epoch: String,
        at: i64,
        #[serde(default, rename = "positionMs")]
        position_ms: f64,
    },
    #[serde(rename = "sync")]
    Sync(Playback),
    #[serde(rename = "snapshot")]
    Snapshot {
        echo: i64,
        at: i64,
        #[serde(default)]
        queue: Option<Vec<WireSong>>,
        #[serde(default)]
        state: Option<Playback>,
    },
    #[serde(rename = "join")]
    Join {
        at: i64,
        #[serde(default)]
        t1: i64,
        from: String,
    },
    #[serde(rename = "queue")]
    Queue { data: String },
    #[serde(rename = "control")]
    Control(Control),
    #[serde(rename = "prepare")]
    Prepare(Playback),
    #[serde(rename = "error")]
    Error { reason: String },
}

impl From<&Song> for WireSong {
    fn from(song: &Song) -> Self {
        let album_id = song.album_id.clone().unwrap_or_default();
        let cover = song.cover.clone();
        Self {
            id: song.id.clone(),
            name: song.name.clone(),
            duration: song.duration,
            url: String::new(),
            share: Share {
                id: share_id(song),
                url: share_url(song),
            },
            album: Album {
                id: album_id,
                title: song.album.clone(),
                url: String::new(),
                covers: cover.into_iter().collect(),
                year: 0,
            },
            artists: song
                .artists
                .iter()
                .filter(|name| !name.is_empty())
                .map(|name| Artist {
                    id: String::new(),
                    name: name.clone(),
                    source: String::new(),
                })
                .collect(),
            youtube_id: song.video().map(str::to_owned),
            broken: song.broken,
        }
    }
}

impl From<WireSong> for Song {
    fn from(song: WireSong) -> Self {
        let source = if song.id.starts_with("spotify:") {
            Source::Spotify
        } else if song.id.starts_with("youtube:") {
            Source::YouTube
        } else {
            Source::Local
        };

        let youtube_id = song
            .youtube_id
            .filter(|id| !id.is_empty())
            .or_else(|| song.id.strip_prefix("youtube:").map(str::to_owned))
            .filter(|id| !id.is_empty());
        Self {
            id: song.id,
            name: song.name,
            artists: song
                .artists
                .into_iter()
                .map(|artist| artist.name)
                .filter(|name| !name.is_empty())
                .collect(),
            album: song.album.title,
            album_id: (!song.album.id.is_empty()).then_some(song.album.id),
            playlist_id: None,
            artist_ids: Vec::new(),
            cover: song.album.covers.into_iter().next(),
            duration: song.duration,
            source,
            video_id: youtube_id,
            uri: None,
            broken: song.broken,
        }
    }
}

pub fn songs(items: &[Song]) -> Vec<WireSong> {
    items.iter().map(WireSong::from).collect()
}

pub fn unwrap_songs(items: Vec<WireSong>) -> Vec<Song> {
    items.into_iter().map(Song::from).collect()
}

fn share_id(song: &Song) -> String {
    if let Some(video) = song.video() {
        return video.to_owned();
    }

    song.id
        .split(':')
        .next_back()
        .unwrap_or(song.id.as_str())
        .to_owned()
}

fn share_url(song: &Song) -> String {
    if let Some(video) = song.video() {
        return format!("https://www.youtube.com/watch?v={video}");
    }

    if let Some(id) = song.id.strip_prefix("spotify:track:") {
        return format!("https://open.spotify.com/track/{id}");
    }

    String::new()
}

#[cfg(test)]
#[path = "wire.test.rs"]
mod tests;
