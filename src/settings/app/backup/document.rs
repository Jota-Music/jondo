//! `jota.backup.json`, the document a Jota library travels in.
//!
//! Jota keeps every device converging through one JSON file in a folder the
//! user syncs with whatever they already use. The shape is Jota's: the sections
//! are read and written here exactly as it writes them, so the same folder can
//! be shared by both apps.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{format::*, sync::*};

impl Document {
    pub(crate) fn parse(text: &str) -> Result<Self, Failure> {
        if text.trim().is_empty() {
            return Ok(Self::default());
        }

        let document: Document =
            serde_json::from_str(text).map_err(|error| Failure::Unreadable(error.to_string()))?;
        if document.version != VERSION {
            return Err(Failure::Version(document.version));
        }

        Ok(document)
    }

    pub(crate) fn empty(&self) -> bool {
        self.playlists.as_ref().is_none_or(|part| part.empty())
            && self.follows.as_ref().is_none_or(|part| part.empty())
            && self.saved.as_ref().is_none_or(|part| part.empty())
            && self.order.as_ref().is_none_or(|part| part.is_empty())
            && self.settings.as_ref().is_none_or(BTreeMap::is_empty)
            && self.sessions.as_ref().is_none_or(BTreeMap::is_empty)
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Document {
    #[serde(default = "version")]
    pub(crate) version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) playlists: Option<Playlists>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) follows: Option<Follows>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) saved: Option<Saved>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) order: Option<Vec<OrderEntry>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) settings: Option<BTreeMap<String, Setting>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) sessions: Option<BTreeMap<String, Setting>>,
}

impl Default for Document {
    fn default() -> Self {
        Self {
            version: VERSION,
            playlists: None,
            follows: None,
            saved: None,
            order: None,
            settings: None,
            sessions: None,
        }
    }
}

pub(crate) fn version() -> u32 {
    VERSION
}

#[derive(Serialize, Deserialize, Default)]
pub(crate) struct Playlists {
    #[serde(default)]
    pub(crate) items: Vec<WirePlaylist>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) songs: BTreeMap<String, WireSong>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) albums: BTreeMap<String, WireAlbum>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) artists: BTreeMap<String, WireArtist>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) deleted: BTreeMap<String, i64>,
}

impl Playlists {
    pub(crate) fn empty(&self) -> bool {
        self.items.is_empty() && self.deleted.is_empty() && self.songs.is_empty()
    }
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub(crate) struct WirePlaylist {
    pub(crate) id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) source: Option<String>,
    #[serde(default)]
    pub(crate) name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) owner: Option<String>,
    #[serde(default, rename = "modifiedAt", skip_serializing_if = "zero")]
    pub(crate) modified_at: i64,
    #[serde(default)]
    pub(crate) songs: Vec<SongRef>,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub(crate) struct SongRef {
    #[serde(default)]
    pub(crate) id: String,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub(crate) struct WireSong {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub(crate) name: String,
    #[serde(default, skip_serializing_if = "zero")]
    pub(crate) duration: i64,
    #[serde(
        default,
        rename = "youtubeId",
        skip_serializing_if = "String::is_empty"
    )]
    pub(crate) youtube_id: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub(crate) broken: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) album: Option<AlbumRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) artists: Vec<ArtistRef>,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub(crate) struct WireAlbum {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub(crate) title: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) covers: Vec<String>,
    #[serde(default, skip_serializing_if = "zero")]
    pub(crate) year: i64,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub(crate) struct AlbumRef {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub(crate) id: String,
    #[serde(flatten)]
    pub(crate) album: WireAlbum,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub(crate) struct WireArtist {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub(crate) name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub(crate) source: String,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub(crate) struct ArtistRef {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub(crate) id: String,
    #[serde(flatten)]
    pub(crate) artist: WireArtist,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub(crate) struct Follows {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) items: Vec<FollowEntry>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) removed: BTreeMap<String, i64>,
}

impl Follows {
    pub(crate) fn empty(&self) -> bool {
        self.items.is_empty() && self.removed.is_empty()
    }
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub(crate) struct FollowEntry {
    #[serde(default)]
    pub(crate) id: String,
    #[serde(default)]
    pub(crate) at: i64,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub(crate) struct Saved {
    #[serde(default)]
    pub(crate) items: Vec<SavedEntry>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) removed: BTreeMap<String, i64>,
}

impl Saved {
    pub(crate) fn empty(&self) -> bool {
        self.items.is_empty() && self.removed.is_empty()
    }
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub(crate) struct SavedEntry {
    #[serde(default)]
    pub(crate) id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub(crate) name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub(crate) cover: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) covers: Vec<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub(crate) subtitle: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub(crate) owner: String,
    #[serde(default)]
    pub(crate) at: i64,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub(crate) struct OrderEntry {
    #[serde(default)]
    pub(crate) scope: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub(crate) account: String,
    #[serde(default)]
    pub(crate) ids: Vec<String>,
    #[serde(default)]
    pub(crate) at: i64,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub(crate) struct Setting {
    #[serde(default)]
    pub(crate) value: Value,
    #[serde(default)]
    pub(crate) at: i64,
}
