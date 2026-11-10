use serde::{Deserialize, Serialize};

use super::{Song, youtube_song};

#[derive(Clone, Debug, Default)]
pub struct Video {
    pub id: String,
    pub title: String,
    pub author: String,
    pub channel: Option<String>,
    pub duration: u32,
    pub official: bool,
}

impl Video {
    pub fn song(&self) -> Song {
        let mut song = youtube_song(&self.id, &self.title, &self.author, self.duration);
        song.artist_ids = self.channel.iter().cloned().collect();
        song
    }
}

/// A signed URL and the headers it was signed for. It is stored as-is: the URL
/// carries its own expiry, so a resolved stream can outlive the process.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Stream {
    pub url: String,
    pub agent: String,
    #[serde(default)]
    pub duration: f64,
}
