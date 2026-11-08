use serde::{Deserialize, Serialize};

use super::{Song, Source};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Playlist {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub subtitle: Option<String>,
    #[serde(default)]
    pub cover: Option<String>,
    #[serde(default)]
    pub covers: Vec<String>,
    #[serde(default)]
    pub source: Source,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub owner_id: Option<String>,
    #[serde(default)]
    pub songs: Vec<Song>,
    #[serde(default)]
    pub track_count: u32,
    #[serde(skip)]
    pub loading: bool,
}

impl Playlist {
    pub fn is_local(&self) -> bool {
        self.id.starts_with("local:")
    }

    /// Liked Songs is a user context, not a playlist the rootlist carries.
    pub fn liked(&self) -> bool {
        self.id.starts_with("spotify:user:") && self.id.ends_with(":collection")
    }
}

#[cfg(test)]
#[path = "playlist.test.rs"]
mod tests;
