use super::{Song, youtube_song};

#[derive(Clone, Debug, Default)]
pub struct Video {
    pub id: String,
    pub title: String,
    pub author: String,
    pub channel: Option<String>,
    pub duration: u32,
}

impl Video {
    pub fn song(&self) -> Song {
        let mut song = youtube_song(&self.id, &self.title, &self.author, self.duration);
        song.artist_ids = self.channel.iter().cloned().collect();
        song
    }
}

#[derive(Clone, Debug, Default)]
pub struct Stream {
    pub url: String,
    pub agent: String,
    pub duration: f64,
}
