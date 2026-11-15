use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::shared::music::domain::{Follow, Playlist, Room, Song};

use super::{
    LocalPlaylist, SpotifyAuth, Store, SyncStatus, enabled, full, full_volume, interval, on,
    sync_import_default, sync_interval_default, zero,
};

/// The shape `state.json` takes on disk. Every section holds what one part of
/// the app owns, so the file reads the way the app is built: the player, the
/// queue, the library, the sessions. A value left at its default is not
/// written, so a state file carries only what the user actually set.
///
/// Both conversions destructure their source, so a new field breaks the build
/// until it is placed here too.
#[derive(Serialize, Deserialize)]
pub(crate) struct Snapshot {
    #[serde(default = "version")]
    version: u32,
    #[serde(default, skip_serializing_if = "zero")]
    player: Player,
    /// The queue moved to its own file in the cache folder. The section is read
    /// for the files written before the move and never written again.
    #[serde(default, skip_serializing)]
    queue: Queue,
    #[serde(default, skip_serializing_if = "zero")]
    library: Library,
    #[serde(default, skip_serializing_if = "zero")]
    follows: Follows,
    #[serde(default, skip_serializing_if = "zero")]
    saved: Saved,
    #[serde(default, skip_serializing_if = "zero")]
    graves: Graves,
    #[serde(default, skip_serializing_if = "zero")]
    settings: Settings,
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

/// What the player is doing: the sound, the place in the song and the mode the
/// queue plays in.
#[derive(PartialEq, Serialize, Deserialize)]
struct Player {
    #[serde(default = "full_volume", skip_serializing_if = "full")]
    volume: f32,
    #[serde(default, skip_serializing_if = "zero")]
    muted: bool,
    #[serde(default, skip_serializing_if = "zero")]
    position: f64,
    #[serde(default, skip_serializing_if = "zero")]
    shuffle: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    repeat: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    last_song: Option<Song>,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            volume: full_volume(),
            muted: false,
            position: 0.0,
            shuffle: false,
            repeat: String::new(),
            last_song: None,
        }
    }
}

/// The songs waiting and the one playing.
#[derive(Default, PartialEq, Serialize, Deserialize)]
struct Queue {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    songs: Vec<Song>,
    #[serde(default, skip_serializing_if = "zero")]
    index: usize,
}

/// The playlists the user made here and the order the shelf shows them in.
#[derive(Default, PartialEq, Serialize, Deserialize)]
struct Library {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    playlists: Vec<LocalPlaylist>,
    #[serde(default, skip_serializing_if = "zero")]
    order: Order,
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

/// What the user set, apart from what a session holds.
#[derive(PartialEq, Serialize, Deserialize)]
struct Settings {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    locale: String,
    #[serde(default, skip_serializing_if = "zero")]
    discord: bool,
    #[serde(default, skip_serializing_if = "zero")]
    compact: bool,
    #[serde(default, skip_serializing_if = "zero")]
    always_on_top: bool,
    #[serde(default = "enabled", skip_serializing_if = "on")]
    backdrop: bool,
    #[serde(default = "sync_import_default", skip_serializing_if = "on")]
    trim_memory: bool,
    #[serde(default = "enabled", skip_serializing_if = "on")]
    pause_on_disconnect: bool,
    #[serde(default = "enabled", skip_serializing_if = "on")]
    pause_on_focus: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            locale: String::new(),
            discord: false,
            compact: false,
            always_on_top: false,
            backdrop: enabled(),
            trim_memory: sync_import_default(),
            pause_on_disconnect: enabled(),
            pause_on_focus: enabled(),
        }
    }
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
    /// The links the state carried before they moved to the cache with each
    /// song's pick. Read once at load and never written again.
    #[serde(default, skip_serializing)]
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

pub(super) fn version() -> u32 {
    2
}

/// The fields the first format kept flat, and the section each one belongs to.
/// A file written before the grouping is moved into its section, so both read
/// the same.
const GROUPS: [(&str, &[&str]); 3] = [
    (
        "player",
        &[
            "volume",
            "muted",
            "position",
            "shuffle",
            "repeat",
            "last_song",
        ],
    ),
    (
        "settings",
        &[
            "locale",
            "discord",
            "compact",
            "always_on_top",
            "backdrop",
            "trim_memory",
            "pause_on_disconnect",
            "pause_on_focus",
        ],
    ),
    ("library", &["playlists", "order"]),
];

fn group(value: &mut Value) {
    let Some(map) = value.as_object_mut() else {
        return;
    };

    let mut moved: Vec<(String, Map<String, Value>)> = Vec::new();

    for (name, keys) in GROUPS {
        let mut fields = Map::new();
        for key in keys {
            if let Some(value) = map.remove(*key) {
                fields.insert((*key).to_owned(), value);
            }
        }
        if !fields.is_empty() {
            moved.push((name.to_owned(), fields));
        }
    }

    // the queue was a bare array next to its index
    let mut queue = Map::new();
    if let Some(Value::Array(songs)) = map.remove("queue") {
        queue.insert("songs".to_owned(), Value::Array(songs));
    }
    if let Some(index) = map.remove("queue_index") {
        queue.insert("index".to_owned(), index);
    }
    if !queue.is_empty() {
        moved.push(("queue".to_owned(), queue));
    }

    for (name, fields) in moved {
        map.insert(name, Value::Object(fields));
    }
}

impl Snapshot {
    /// Reads the file. A file written before the sections were grouped holds
    /// the same values flat, so they are moved into their section first.
    pub(crate) fn from_text(text: &str) -> Result<Self, serde_json::Error> {
        let mut value: Value = serde_json::from_str(text)?;
        if value.get("version").and_then(Value::as_u64).unwrap_or(0) < version() as u64 {
            group(&mut value);
        }
        serde_json::from_value(value)
    }

    /// The links the state carried before the cache owned them.
    pub(crate) fn links(&self) -> BTreeMap<String, String> {
        self.youtube.links.clone()
    }
}

impl From<&Store> for Snapshot {
    fn from(store: &Store) -> Self {
        let Store {
            playlists,
            volume,
            muted,
            last_song,
            queue: _,
            queue_index: _,
            position,
            spotify,
            spotify_device,
            shuffle,
            repeat,
            compact,
            always_on_top,
            backdrop,
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
            youtube_picks,
        } = store;

        Self {
            version: version(),
            player: Player {
                volume: *volume,
                muted: *muted,
                position: *position,
                shuffle: *shuffle,
                repeat: repeat.clone(),
                last_song: last_song.clone(),
            },
            queue: Queue {
                songs: Vec::new(),
                index: 0,
            },
            library: Library {
                playlists: playlists.clone(),
                order: Order {
                    ids: order.clone(),
                    at: *order_at,
                },
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
            settings: Settings {
                locale: locale.clone(),
                discord: *discord,
                compact: *compact,
                always_on_top: *always_on_top,
                backdrop: *backdrop,
                trim_memory: *trim_memory,
                pause_on_disconnect: *pause_on_disconnect,
                pause_on_focus: *pause_on_focus,
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
                links: BTreeMap::new(),
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
            version: _,
            player,
            queue,
            library,
            follows,
            saved,
            graves,
            settings,
            spotify,
            youtube,
            sync,
            relay,
            rooms,
        } = snapshot;

        Self {
            playlists: library.playlists,
            volume: player.volume,
            muted: player.muted,
            last_song: player.last_song,
            queue: queue.songs,
            queue_index: queue.index,
            position: player.position,
            spotify: spotify.auth,
            spotify_device: spotify.device,
            shuffle: player.shuffle,
            repeat: player.repeat,
            compact: settings.compact,
            always_on_top: settings.always_on_top,
            backdrop: settings.backdrop,
            order: library.order.ids,
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
            order_at: library.order.at,
            room_at: rooms.at,
            relay_at: relay.at,
            playlist_graves: graves.playlists,
            follow_graves: graves.follows,
            saved_graves: graves.saved,
            follows_at: follows.at,
            locale: settings.locale,
            discord: settings.discord,
            youtube_cookies: youtube.cookies,
            youtube_full: youtube.full,
            youtube_browser: youtube.browser,
            youtube_at: youtube.at,
            spotify_at: spotify.at,
            session_pass: sync.pass,
            trim_memory: settings.trim_memory,
            pause_on_disconnect: settings.pause_on_disconnect,
            pause_on_focus: settings.pause_on_focus,
            saved_at: saved.at,
            youtube_picks: youtube.picks,
        }
    }
}
