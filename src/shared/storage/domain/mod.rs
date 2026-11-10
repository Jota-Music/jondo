use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub mod report;
mod snapshot;

use crate::shared::music::domain::Room;
use crate::shared::music::domain::{Playlist, Song};

pub use report::SyncStatus;
pub(super) use snapshot::Snapshot;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LocalPlaylist {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub songs: Vec<Song>,
    #[serde(default, skip_serializing_if = "zero")]
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
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
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

/// True when a field holds the value it would get without being written, so a
/// state file carries only what the user actually set.
pub(super) fn zero<T: Default + PartialEq>(value: &T) -> bool {
    *value == T::default()
}

pub(super) fn full(value: &f32) -> bool {
    *value == full_volume()
}

pub(super) fn on(value: &bool) -> bool {
    *value
}

pub(super) fn interval(value: &u64) -> bool {
    *value == sync_interval_default()
}

#[derive(Clone, Debug, PartialEq)]
pub struct Store {
    pub playlists: Vec<LocalPlaylist>,
    pub volume: f32,
    pub muted: bool,
    pub last_song: Option<Song>,
    pub queue: Vec<Song>,
    pub queue_index: usize,
    pub position: f64,
    pub spotify: Option<SpotifyAuth>,
    pub spotify_device: String,
    pub shuffle: bool,
    pub repeat: String,
    pub compact: bool,
    pub always_on_top: bool,
    pub order: Vec<String>,
    pub follows: Vec<crate::shared::music::domain::Follow>,
    pub saved: Vec<Playlist>,
    pub relay_url: String,
    pub relay_token: String,
    pub rooms: Vec<Room>,
    pub room_order: Vec<String>,
    pub room_code: String,
    pub room_password: String,
    pub sync_folder: String,
    pub sync_import: bool,
    pub sync_auto: bool,
    pub sync_interval: u64,
    pub sync_pending: bool,
    pub sync_status: SyncStatus,
    pub order_at: i64,
    pub room_at: i64,
    pub relay_at: i64,
    pub playlist_graves: BTreeMap<String, i64>,
    pub follow_graves: BTreeMap<String, i64>,
    pub saved_graves: BTreeMap<String, i64>,
    pub follows_at: BTreeMap<String, i64>,
    pub locale: String,
    pub discord: bool,
    pub youtube_cookies: String,
    pub youtube_full: bool,
    /// The browser the session was imported from, so a launch can read it again
    /// instead of asking which one to use every time.
    pub youtube_browser: String,
    pub youtube_at: i64,
    pub spotify_at: i64,
    pub session_pass: String,
    pub trim_memory: bool,
    pub pause_on_disconnect: bool,
    pub pause_on_focus: bool,
    pub saved_at: BTreeMap<String, i64>,
    pub youtube_links: BTreeMap<String, String>,
    pub youtube_picks: BTreeSet<String>,
}

pub(super) fn sync_import_default() -> bool {
    true
}

pub(super) fn enabled() -> bool {
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
