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
    /// The ids the playlist carries, in the names the app writes now.
    pub fn normalize(&mut self) {
        self.id = super::id::playlist(&self.id);

        if let Some(owner) = self.owner_id.clone() {
            self.owner_id = Some(super::id::channel(&owner));
        }

        for song in self.songs.iter_mut() {
            song.normalize();
        }
    }

    pub fn is_local(&self) -> bool {
        self.id.starts_with("local:playlist:")
    }

    /// Liked Songs is a user context, not a playlist the rootlist carries.
    pub fn liked(&self) -> bool {
        self.id.starts_with("spotify:collection:")
    }

    /// Song radio is a synthetic context, not a playlist that can be followed.
    pub fn radio(&self) -> bool {
        self.id.starts_with("spotify:radio:")
    }
}

#[cfg(test)]
#[path = "playlist.test.rs"]
mod tests;
