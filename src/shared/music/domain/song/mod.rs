use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    Spotify,
    YouTube,
    #[default]
    Local,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Song {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub artists: Vec<String>,
    #[serde(default)]
    pub album: String,
    #[serde(default)]
    pub album_id: Option<String>,
    #[serde(default)]
    pub playlist_id: Option<String>,
    #[serde(default)]
    pub artist_ids: Vec<String>,
    #[serde(default)]
    pub cover: Option<String>,
    #[serde(default)]
    pub duration: u32,
    #[serde(default)]
    pub source: Source,
    #[serde(default)]
    pub video_id: Option<String>,
    #[serde(default)]
    pub uri: Option<String>,
    #[serde(default)]
    pub broken: bool,
}

impl Song {
    pub fn subtitle(&self) -> String {
        if self.artists.is_empty() {
            self.album.clone()
        } else {
            self.artists.join(", ")
        }
    }

    pub fn query(&self) -> String {
        let mut query = self.name.clone();
        if let Some(artist) = self.artists.first()
            && !artist.is_empty()
        {
            query.push(' ');
            query.push_str(artist);
        }

        query
    }

    pub fn video(&self) -> Option<&str> {
        self.video_id
            .as_deref()
            .or_else(|| {
                self.uri
                    .as_deref()
                    .and_then(|value| value.strip_prefix("youtube:"))
                    .filter(|value| !value.is_empty())
            })
            .or_else(|| {
                self.id
                    .strip_prefix("youtube:")
                    .filter(|value| !value.is_empty())
            })
    }
}

pub fn local_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis() as u64)
        .unwrap_or_default();
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("local:{now:x}{count:x}")
}

pub fn youtube_song(video_id: &str, title: &str, author: &str, duration: u32) -> Song {
    Song {
        id: format!("youtube:{video_id}"),
        name: title.to_owned(),
        artists: if author.is_empty() {
            Vec::new()
        } else {
            vec![author.to_owned()]
        },
        album: String::new(),
        album_id: None,
        playlist_id: None,
        artist_ids: Vec::new(),
        cover: Some(format!("https://i.ytimg.com/vi/{video_id}/mqdefault.jpg")),
        duration,
        source: Source::YouTube,
        video_id: Some(video_id.to_owned()),
        uri: None,
        broken: false,
    }
}

#[cfg(test)]
#[path = "song.test.rs"]
mod tests;
