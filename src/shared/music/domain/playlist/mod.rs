use serde::{Deserialize, Serialize};

use super::{Song, Source};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Playlist {
    pub id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub covers: Vec<String>,
    #[serde(default, skip_serializing_if = "is_local")]
    pub source: Source,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub songs: Vec<Song>,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub track_count: u32,
    #[serde(skip)]
    pub loading: bool,
}

fn is_zero(value: &u32) -> bool {
    *value == 0
}

fn is_local(source: &Source) -> bool {
    *source == Source::Local
}

impl Playlist {
    pub fn is_local(&self) -> bool {
        self.id.starts_with("local:")
    }

    /// Liked Songs is a user context, not a playlist the rootlist carries.
    pub fn liked(&self) -> bool {
        self.id.starts_with("spotify:user:") && self.id.ends_with(":collection")
    }

    /// Song radio is a synthetic context, not a playlist that can be followed.
    pub fn radio(&self) -> bool {
        self.id.starts_with("spotify:radio:")
    }
}

#[cfg(test)]
#[path = "playlist.test.rs"]
mod tests;
