use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::shared::music::domain::{Follow, Playlist, Room, Song};

use super::{
    LocalPlaylist, SpotifyAuth, Store, SyncStatus, enabled, full, full_volume, interval, on,
    sync_import_default, sync_interval_default, zero,
};

/// The shape `state.json` takes on disk. The store keeps these values flat
/// because every feature reads them directly; the file groups what belongs
/// together, so a section is one object instead of a row of `youtube_*` names.
/// Both conversions destructure their source, so a new field breaks the build
/// until it is placed here too.
#[derive(Serialize, Deserialize)]
pub(crate) struct Snapshot {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    playlists: Vec<LocalPlaylist>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    queue: Vec<Song>,
    #[serde(default, skip_serializing_if = "zero")]
    queue_index: usize,
    #[serde(default = "full_volume", skip_serializing_if = "full")]
    volume: f32,
    #[serde(default, skip_serializing_if = "zero")]
    muted: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    last_song: Option<Song>,
    #[serde(default, skip_serializing_if = "zero")]
    position: f64,
    #[serde(default, skip_serializing_if = "zero")]
    shuffle: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    repeat: String,
    #[serde(default, skip_serializing_if = "zero")]
    compact: bool,
    #[serde(default, skip_serializing_if = "zero")]
    always_on_top: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    locale: String,
    #[serde(default, skip_serializing_if = "zero")]
    discord: bool,
    #[serde(default = "sync_import_default", skip_serializing_if = "on")]
    trim_memory: bool,
    #[serde(default = "enabled", skip_serializing_if = "on")]
    pause_on_disconnect: bool,
    #[serde(default = "enabled", skip_serializing_if = "on")]
    pause_on_focus: bool,
    #[serde(default, skip_serializing_if = "zero")]
    order: Order,
    #[serde(default, skip_serializing_if = "zero")]
    follows: Follows,
    #[serde(default, skip_serializing_if = "zero")]
    saved: Saved,
    #[serde(default, skip_serializing_if = "zero")]
    graves: Graves,
    #[serde(default, skip_serializing_if = "zero")]
    spotify: Spotify,
    #[serde(default, skip_serializing_if = "zero")]
    youtube: Youtube,
    #[serde(default, skip_serializing_if = "zero")]
    sync: Sync,
    #[serde(default, skip_serializing_if = "zero")]
    relay: Relay,
    #[serde(default, skip_serializing_if = "zero")]
    rooms: Rooms,
}

/// The shelf order and the time it last changed.
#[derive(Default, PartialEq, Serialize, Deserialize)]
struct Order {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    ids: Vec<String>,
    #[serde(default, skip_serializing_if = "zero")]
    at: i64,
}

#[derive(Default, PartialEq, Serialize, Deserialize)]
struct Follows {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    items: Vec<Follow>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    at: BTreeMap<String, i64>,
}

#[derive(Default, PartialEq, Serialize, Deserialize)]
struct Saved {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    items: Vec<Playlist>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    at: BTreeMap<String, i64>,
}

/// The ids removed locally, with the time of the removal, so a merge does not
/// bring them back from another device.
#[derive(Default, PartialEq, Serialize, Deserialize)]
struct Graves {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    playlists: BTreeMap<String, i64>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    follows: BTreeMap<String, i64>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    saved: BTreeMap<String, i64>,
}

#[derive(Default, PartialEq, Serialize, Deserialize)]
struct Spotify {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    auth: Option<SpotifyAuth>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    device: String,
    #[serde(default, skip_serializing_if = "zero")]
    at: i64,
}

#[derive(Default, PartialEq, Serialize, Deserialize)]
struct Youtube {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    cookies: String,
    #[serde(default, skip_serializing_if = "zero")]
    full: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    browser: String,
    #[serde(default, skip_serializing_if = "zero")]
    at: i64,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    links: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    picks: BTreeSet<String>,
}

#[derive(PartialEq, Serialize, Deserialize)]
struct Sync {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    folder: String,
    #[serde(default = "sync_import_default", skip_serializing_if = "on")]
    import: bool,
    #[serde(default = "sync_import_default", skip_serializing_if = "on")]
    auto: bool,
    #[serde(default = "sync_interval_default", skip_serializing_if = "interval")]
    interval: u64,
    #[serde(default, skip_serializing_if = "zero")]
    pending: bool,
    #[serde(default, skip_serializing_if = "zero")]
    status: SyncStatus,
    /// The pass this device holds for the sessions in the document.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pass: String,
}

impl Default for Sync {
    fn default() -> Self {
        Self {
            folder: String::new(),
            import: sync_import_default(),
            auto: sync_import_default(),
            interval: sync_interval_default(),
            pending: false,
            status: SyncStatus::default(),
            pass: String::new(),
        }
    }
}

#[derive(Default, PartialEq, Serialize, Deserialize)]
struct Relay {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    url: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    token: String,
    #[serde(default, skip_serializing_if = "zero")]
    at: i64,
}

#[derive(Default, PartialEq, Serialize, Deserialize)]
struct Rooms {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    items: Vec<Room>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    order: Vec<String>,
    #[serde(default, skip_serializing_if = "zero")]
    at: i64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    code: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    password: String,
}

impl From<&Store> for Snapshot {
    fn from(store: &Store) -> Self {
        let Store {
            playlists,
            volume,
            muted,
            last_song,
            queue,
            queue_index,
            position,
            spotify,
            spotify_device,
            shuffle,
            repeat,
            compact,
            always_on_top,
            order,
            follows,
            saved,
            relay_url,
            relay_token,
            rooms,
            room_order,
            room_code,
            room_password,
            sync_folder,
            sync_import,
            sync_auto,
            sync_interval,
            sync_pending,
            sync_status,
            order_at,
            room_at,
            relay_at,
            playlist_graves,
            follow_graves,
            saved_graves,
            follows_at,
            locale,
            discord,
            youtube_cookies,
            youtube_full,
            youtube_browser,
            youtube_at,
            spotify_at,
            session_pass,
            trim_memory,
            pause_on_disconnect,
            pause_on_focus,
            saved_at,
            youtube_links,
            youtube_picks,
        } = store;

        Self {
            playlists: playlists.clone(),
            queue: queue.clone(),
            queue_index: *queue_index,
            volume: *volume,
            muted: *muted,
            last_song: last_song.clone(),
            position: *position,
            shuffle: *shuffle,
            repeat: repeat.clone(),
            compact: *compact,
            always_on_top: *always_on_top,
            locale: locale.clone(),
            discord: *discord,
            trim_memory: *trim_memory,
            pause_on_disconnect: *pause_on_disconnect,
            pause_on_focus: *pause_on_focus,
            order: Order {
                ids: order.clone(),
                at: *order_at,
            },
            follows: Follows {
                items: follows.clone(),
                at: follows_at.clone(),
            },
            saved: Saved {
                items: saved.clone(),
                at: saved_at.clone(),
            },
            graves: Graves {
                playlists: playlist_graves.clone(),
                follows: follow_graves.clone(),
                saved: saved_graves.clone(),
            },
            spotify: Spotify {
                auth: spotify.clone(),
                device: spotify_device.clone(),
                at: *spotify_at,
            },
            youtube: Youtube {
                cookies: youtube_cookies.clone(),
                full: *youtube_full,
                browser: youtube_browser.clone(),
                at: *youtube_at,
                links: youtube_links.clone(),
                picks: youtube_picks.clone(),
            },
            sync: Sync {
                folder: sync_folder.clone(),
                import: *sync_import,
                auto: *sync_auto,
                interval: *sync_interval,
                pending: *sync_pending,
                status: sync_status.clone(),
                pass: session_pass.clone(),
            },
            relay: Relay {
                url: relay_url.clone(),
                token: relay_token.clone(),
                at: *relay_at,
            },
            rooms: Rooms {
                items: rooms.clone(),
                order: room_order.clone(),
                at: *room_at,
                code: room_code.clone(),
                password: room_password.clone(),
            },
        }
    }
}

impl From<Snapshot> for Store {
    fn from(snapshot: Snapshot) -> Self {
        let Snapshot {
            playlists,
            queue,
            queue_index,
            volume,
            muted,
            last_song,
            position,
            shuffle,
            repeat,
            compact,
            always_on_top,
            locale,
            discord,
            trim_memory,
            pause_on_disconnect,
            pause_on_focus,
            order,
            follows,
            saved,
            graves,
            spotify,
            youtube,
            sync,
            relay,
            rooms,
        } = snapshot;

        Self {
            playlists,
            volume,
            muted,
            last_song,
            queue,
            queue_index,
            position,
            spotify: spotify.auth,
            spotify_device: spotify.device,
            shuffle,
            repeat,
            compact,
            always_on_top,
            order: order.ids,
            follows: follows.items,
            saved: saved.items,
            relay_url: relay.url,
            relay_token: relay.token,
            rooms: rooms.items,
            room_order: rooms.order,
            room_code: rooms.code,
            room_password: rooms.password,
            sync_folder: sync.folder,
            sync_import: sync.import,
            sync_auto: sync.auto,
            sync_interval: sync.interval,
            sync_pending: sync.pending,
            sync_status: sync.status,
            order_at: order.at,
            room_at: rooms.at,
            relay_at: relay.at,
            playlist_graves: graves.playlists,
            follow_graves: graves.follows,
            saved_graves: graves.saved,
            follows_at: follows.at,
            locale,
            discord,
            youtube_cookies: youtube.cookies,
            youtube_full: youtube.full,
            youtube_browser: youtube.browser,
            youtube_at: youtube.at,
            spotify_at: spotify.at,
            session_pass: sync.pass,
            trim_memory,
            pause_on_disconnect,
            pause_on_focus,
            saved_at: saved.at,
            youtube_links: youtube.links,
            youtube_picks: youtube.picks,
        }
    }
}
