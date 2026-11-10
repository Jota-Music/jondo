use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub mod report;

use crate::shared::music::domain::Room;
use crate::shared::music::domain::{Playlist, Song};

pub use report::SyncStatus;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LocalPlaylist {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub songs: Vec<Song>,
    #[serde(default)]
    pub modified: u64,
}

impl LocalPlaylist {
    pub fn covers(&self) -> Vec<String> {
        let mut covers: Vec<String> = Vec::with_capacity(4);
        for cover in self.songs.iter().filter_map(|song| song.cover.as_deref()) {
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
}

/// The session Jota stores: the account and the reusable credentials the
/// accesspoint hands back at login. Jondo keeps the same pair, so either app can
/// carry the other's session through the backup.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SpotifyAuth {
    /// Declared before `username` so the JSON is byte-identical to Jota's, which
    /// marshals a map and therefore sorts the keys. Two apps sharing one backup
    /// would otherwise re-encrypt each other's session on every pass.
    #[serde(default)]
    pub data: String,
    #[serde(default)]
    pub username: String,
}

impl SpotifyAuth {
    pub fn signed_in(&self) -> bool {
        !self.username.is_empty() && !self.data.is_empty()
    }
}

pub(super) fn full_volume() -> f32 {
    1.0
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Store {
    #[serde(default)]
    pub playlists: Vec<LocalPlaylist>,
    #[serde(default = "full_volume")]
    pub volume: f32,
    #[serde(default)]
    pub muted: bool,
    #[serde(default)]
    pub last_song: Option<Song>,
    #[serde(default)]
    pub queue: Vec<Song>,
    #[serde(default)]
    pub queue_index: usize,
    #[serde(default)]
    pub position: f64,
    #[serde(default)]
    pub spotify: Option<SpotifyAuth>,
    #[serde(default)]
    pub spotify_device: String,
    #[serde(default)]
    pub shuffle: bool,
    #[serde(default)]
    pub repeat: String,
    #[serde(default)]
    pub compact: bool,
    #[serde(default)]
    pub always_on_top: bool,
    #[serde(default)]
    pub order: Vec<String>,
    #[serde(default)]
    pub follows: Vec<crate::shared::music::domain::Follow>,
    #[serde(default)]
    pub saved: Vec<Playlist>,
    #[serde(default)]
    pub relay_url: String,
    #[serde(default)]
    pub relay_token: String,
    #[serde(default)]
    pub rooms: Vec<Room>,
    #[serde(default)]
    pub room_order: Vec<String>,
    #[serde(default)]
    pub room_code: String,
    #[serde(default)]
    pub room_password: String,
    #[serde(default)]
    pub sync_folder: String,
    #[serde(default = "sync_import_default")]
    pub sync_import: bool,
    #[serde(default = "sync_import_default")]
    pub sync_auto: bool,
    #[serde(default = "sync_interval_default")]
    pub sync_interval: u64,
    #[serde(default)]
    pub sync_pending: bool,
    #[serde(default)]
    pub sync_status: SyncStatus,
    #[serde(default)]
    pub order_at: i64,
    #[serde(default)]
    pub room_at: i64,
    #[serde(default)]
    pub relay_at: i64,
    #[serde(default)]
    pub playlist_graves: BTreeMap<String, i64>,
    #[serde(default)]
    pub follow_graves: BTreeMap<String, i64>,
    #[serde(default)]
    pub saved_graves: BTreeMap<String, i64>,
    #[serde(default)]
    pub follows_at: BTreeMap<String, i64>,
    #[serde(default)]
    pub locale: String,
    #[serde(default)]
    pub discord: bool,
    #[serde(default)]
    pub youtube_cookies: String,
    #[serde(default)]
    pub youtube_full: bool,
    /// The browser the session was imported from, so a launch can read it again
    /// instead of asking which one to use every time.
    #[serde(default)]
    pub youtube_browser: String,
    #[serde(default)]
    pub youtube_at: i64,
    #[serde(default)]
    pub spotify_at: i64,
    #[serde(default)]
    pub session_pass: String,
    #[serde(default = "sync_import_default")]
    pub trim_memory: bool,
    #[serde(default = "enabled")]
    pub pause_on_disconnect: bool,
    #[serde(default = "enabled")]
    pub pause_on_focus: bool,
    #[serde(default)]
    pub saved_at: BTreeMap<String, i64>,
    #[serde(default)]
    pub youtube_links: BTreeMap<String, String>,
    #[serde(default)]
    pub youtube_picks: BTreeSet<String>,
}

pub(super) fn sync_import_default() -> bool {
    true
}

fn enabled() -> bool {
    true
}

pub(super) fn sync_interval_default() -> u64 {
    15
}

impl Default for Store {
    fn default() -> Self {
        Self {
            playlists: Vec::new(),
            volume: 1.0,
            muted: false,
            last_song: None,
            queue: Vec::new(),
            queue_index: 0,
            position: 0.0,
            spotify: None,
            spotify_device: String::new(),
            shuffle: false,
            repeat: String::new(),
            compact: false,
            always_on_top: false,
            order: Vec::new(),
            follows: Vec::new(),
            saved: Vec::new(),
            relay_url: String::new(),
            relay_token: String::new(),
            rooms: Vec::new(),
            room_order: Vec::new(),
            room_code: String::new(),
            room_password: String::new(),
            sync_folder: String::new(),
            sync_import: true,
            sync_auto: true,
            sync_interval: 15,
            sync_pending: false,
            sync_status: SyncStatus::default(),
            order_at: 0,
            room_at: 0,
            relay_at: 0,
            playlist_graves: BTreeMap::new(),
            follow_graves: BTreeMap::new(),
            saved_graves: BTreeMap::new(),
            follows_at: BTreeMap::new(),
            saved_at: BTreeMap::new(),
            youtube_links: BTreeMap::new(),
            youtube_picks: BTreeSet::new(),
            locale: String::new(),
            discord: false,
            youtube_cookies: String::new(),
            youtube_full: false,
            youtube_browser: String::new(),
            youtube_at: 0,
            spotify_at: 0,
            session_pass: String::new(),
            trim_memory: true,
            pause_on_disconnect: true,
            pause_on_focus: true,
        }
    }
}
