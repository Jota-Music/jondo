use std::fs;

use crate::shared::music::domain::Room;

use crate::shared::storage::domain::*;
use crate::shared::storage::links;

use crate::shared::music::domain::{Follow, FollowKind, Playlist, Song, Source};

#[test]
fn an_unreadable_state_is_kept_instead_of_erased() {
    let dir = std::env::temp_dir().join(format!("jondo-state-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("state.json");
    fs::write(&path, "{not json").unwrap();

    let store = Store::load_from(&path);

    assert!(store.queue.is_empty());
    assert!(!path.exists());
    assert_eq!(
        fs::read_to_string(dir.join("state.bad")).unwrap(),
        "{not json"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_default_store_writes_only_the_version() {
    let text = serde_json::to_string(&Snapshot::from(&Store::default())).unwrap();
    assert_eq!(text, r#"{"version":2}"#);
}

#[test]
fn the_state_groups_the_player_and_leaves_the_queue_to_the_cache() {
    let store = Store {
        volume: 0.4,
        queue: vec![Song {
            id: "spotify:track:one".to_owned(),
            ..Default::default()
        }],
        queue_index: 2,
        ..Default::default()
    };

    let text = serde_json::to_string(&Snapshot::from(&store)).unwrap();

    assert!(text.contains(r#""player":{"volume":0.4}"#), "{text}");
    assert!(
        !text.contains("queue"),
        "the queue is not in the state file"
    );
}

#[test]
fn a_flat_state_reads_the_same_as_the_grouped_one() {
    let legacy = r#"{"volume":0.5,"muted":true,"position":12.5,"shuffle":true,"repeat":"all","last_song":{"id":"spotify:track:one"},"queue":[{"id":"spotify:track:two"}],"queue_index":3,"compact":true,"locale":"es","discord":true,"trim_memory":false,"pause_on_disconnect":false,"pause_on_focus":false,"playlists":[{"id":"local:one","name":"Mix"}],"order":{"ids":["spotify:playlist:one"],"at":4}}"#;

    let store = Store::from(Snapshot::from_text(legacy).unwrap());

    assert_eq!(store.volume, 0.5);
    assert!(store.muted);
    assert_eq!(store.position, 12.5);
    assert!(store.shuffle);
    assert_eq!(store.repeat, "all");
    assert_eq!(
        store.last_song.as_ref().map(|song| song.id.as_str()),
        Some("spotify:track:one")
    );
    assert_eq!(store.queue.len(), 1);
    assert_eq!(store.queue_index, 3);
    assert!(store.compact);
    assert_eq!(store.locale, "es");
    assert!(store.discord);
    assert!(!store.trim_memory);
    assert!(!store.pause_on_disconnect);
    assert!(!store.pause_on_focus);
    assert_eq!(store.playlists.len(), 1);
    assert_eq!(store.order, ["spotify:playlist:one".to_owned()]);
    assert_eq!(store.order_at, 4);
}

#[test]
fn a_song_writes_only_the_fields_it_holds() {
    let song = Song {
        id: "youtube:abc".to_owned(),
        name: "Title".to_owned(),
        source: Source::YouTube,
        ..Default::default()
    };

    assert_eq!(
        serde_json::to_string(&song).unwrap(),
        r#"{"id":"youtube:abc","name":"Title","source":"youtube"}"#
    );
}

#[test]
fn a_default_song_writes_only_its_id() {
    assert_eq!(
        serde_json::to_string(&Song::default()).unwrap(),
        r#"{"id":""}"#
    );
}

#[test]
fn the_snapshot_keeps_every_section() {
    let mut store = Store::default();
    store.playlists.push(LocalPlaylist {
        id: "local:one".to_owned(),
        name: "Mix".to_owned(),
        songs: vec![Song {
            id: "spotify:track:one".to_owned(),
            ..Default::default()
        }],
        modified: 1,
    });
    store.volume = 0.4;
    store.muted = true;
    store.last_song = Some(Song {
        id: "spotify:track:two".to_owned(),
        ..Default::default()
    });
    store.queue.push(Song {
        id: "spotify:track:three".to_owned(),
        ..Default::default()
    });
    store.queue_index = 2;
    store.position = 12.5;
    store.spotify = Some(SpotifyAuth {
        data: "data".to_owned(),
        username: "user".to_owned(),
    });
    store.spotify_device = "device".to_owned();
    store.shuffle = true;
    store.repeat = "all".to_owned();
    store.compact = true;
    store.always_on_top = true;
    store.backdrop = false;
    store.order = vec!["spotify:playlist:one".to_owned()];
    store.follows.push(Follow {
        id: "artist:one".to_owned(),
        ..Default::default()
    });
    store.saved.push(Playlist {
        id: "spotify:playlist:one".to_owned(),
        ..Default::default()
    });
    store.relay_url = "relay".to_owned();
    store.relay_token = "token".to_owned();
    store.rooms.push(Room {
        code: "consejo".to_owned(),
        ..Default::default()
    });
    store.room_order = vec!["relay|consejo".to_owned()];
    store.room_code = "consejo".to_owned();
    store.room_password = "secret".to_owned();
    store.sync_folder = "/sync".to_owned();
    store.sync_import = false;
    store.sync_auto = false;
    store.sync_interval = 60;
    store.sync_pending = true;
    store.sync_status.at = 3;
    store.order_at = 4;
    store.room_at = 5;
    store.relay_at = 6;
    store.playlist_graves.insert("local:gone".to_owned(), 7);
    store.follow_graves.insert("artist:gone".to_owned(), 8);
    store
        .saved_graves
        .insert("spotify:playlist:gone".to_owned(), 9);
    store.follows_at.insert("artist:one".to_owned(), 10);
    store.locale = "es".to_owned();
    store.discord = true;
    store.youtube_cookies = "SAPISID=abc".to_owned();
    store.youtube_full = true;
    store.youtube_browser = "Zen".to_owned();
    store.youtube_at = 11;
    store.spotify_at = 12;
    store.session_pass = "pass".to_owned();
    store.trim_memory = false;
    store.pause_on_disconnect = false;
    store.pause_on_focus = false;
    store.saved_at.insert("spotify:playlist:one".to_owned(), 13);
    store.youtube_picks.insert("spotify:track:one".to_owned());

    let text = serde_json::to_string(&Snapshot::from(&store)).unwrap();
    let back = Store::from(Snapshot::from_text(&text).unwrap());

    // the queue travels in its own file, so it is not in the state file
    let mut expected = store.clone();
    expected.queue.clear();
    expected.queue_index = 0;
    assert_eq!(expected, back);
    assert!(
        text.contains(r#""youtube":{"cookies":"SAPISID=abc""#),
        "{text}"
    );
}

#[test]
fn a_state_without_a_browser_keeps_the_session() {
    let store = Store::from(
        Snapshot::from_text(r#"{"youtube":{"cookies":"SAPISID=abc","full":true}}"#).unwrap(),
    );
    assert!(store.youtube_browser.is_empty());
    assert_eq!(store.youtube_cookies, "SAPISID=abc");
    assert!(store.youtube_full);
}

#[test]
fn links_skip_native_ids_and_clear_on_empty() {
    let mut store = Store::default();
    assert!(store.link("spotify:track:abc", "dQw4w9WgXcQ"));
    assert!(!store.link("spotify:track:abc", "dQw4w9WgXcQ"));
    assert_eq!(
        links::read("spotify:track:abc").as_deref(),
        Some("youtube:track:dQw4w9WgXcQ")
    );
    assert!(store.pick("spotify:track:abc", "dQw4w9WgXcQ"));
    assert!(store.youtube_picks.contains("spotify:track:abc"));
    assert!(!store.link("spotify:track:abc", "other"));
    assert_eq!(
        links::read("spotify:track:abc").as_deref(),
        Some("youtube:track:dQw4w9WgXcQ")
    );
    assert!(store.link("spotify:track:abc", ""));
    assert!(links::read("spotify:track:abc").is_none());
    assert!(store.youtube_picks.is_empty());
    assert!(!store.pick("youtube:track:dQw4w9WgXcQ", "dQw4w9WgXcQ"));
    assert!(store.youtube_picks.is_empty());
}

/// A pick is cache: the state file carries the pins the user fixed and nothing
/// else, so a learned link never lands in the backup as if it were one.
#[test]
fn the_state_keeps_pins_and_leaves_the_picks_to_the_cache() {
    let mut store = Store::default();
    store.link("spotify:track:learned", "dQw4w9WgXcQ");
    store.pick("spotify:track:pinned", "dQw4w9WgXcQ");

    let text = serde_json::to_string(&Snapshot::from(&store)).unwrap();
    assert!(!text.contains("links"), "{text}");
    assert!(text.contains("picks"), "{text}");
}

#[test]
fn saved_rooms_follow_the_shelf_order() {
    let mut store = Store::default();
    let room = |code: &str| Room {
        id: crate::shared::music::domain::room_id("relay", code),
        code: code.to_owned(),
        relay_url: "relay".to_owned(),
        ..Default::default()
    };

    store.rooms.push(room("alpha"));
    store.rooms.push(room("beta"));
    store.rooms.push(room("gamma"));
    store.room_order = vec![room("beta").id, room("alpha").id];

    let ordered = store.rooms_ordered();
    let codes: Vec<&str> = ordered.iter().map(|room| room.code.as_str()).collect();
    assert_eq!(codes, ["beta", "alpha", "gamma"]);
}

#[test]
fn a_local_playlist_mosaic_keeps_four_distinct_covers() {
    let song = |cover: Option<&str>| crate::shared::music::domain::Song {
        cover: cover.map(str::to_owned),
        ..Default::default()
    };
    let playlist = LocalPlaylist {
        id: "local:one".to_owned(),
        name: "Mix".to_owned(),
        songs: vec![
            song(Some("a")),
            song(None),
            song(Some("a")),
            song(Some("b")),
            song(Some("c")),
            song(Some("d")),
            song(Some("e")),
        ],
        modified: 0,
    };

    assert_eq!(playlist.covers(), ["a", "b", "c", "d"]);
}

#[test]
fn the_queue_travels_in_its_own_file() {
    let dir = std::env::temp_dir().join(format!("jondo-queue-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("state.json");

    let mut store = Store::default();
    store.queue.push(Song {
        id: "spotify:track:one".to_owned(),
        ..Default::default()
    });
    store.queue_index = 1;
    store.write_to(&path);

    let state = fs::read_to_string(&path).unwrap();
    assert!(!state.contains("queue"), "the state file has no queue");
    let queue = fs::read_to_string(dir.join("cache").join("queue.json")).unwrap();
    assert!(queue.contains("spotify:track:one"), "{queue}");

    let back = Store::load_from(&path);
    assert_eq!(back.queue.len(), 1);
    assert_eq!(back.queue_index, 1);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn the_old_ids_move_to_the_new_names() {
    let mut store = Store::default();
    store.queue.push(Song {
        id: "youtube:abc".to_owned(),
        album_id: Some("fedcba9876543210fedcba9876543210".to_owned()),
        artist_ids: vec!["youtube:UC1".to_owned()],
        ..Default::default()
    });
    store.playlists.push(LocalPlaylist {
        id: "local:1a".to_owned(),
        name: "Mix".to_owned(),
        songs: vec![Song {
            id: "spotify:track:abc".to_owned(),
            ..Default::default()
        }],
        modified: 1,
    });
    store.order = vec!["local:1a".to_owned()];
    store.rooms.push(Room {
        id: "relay|code".to_owned(),
        code: "code".to_owned(),
        ..Default::default()
    });

    store.normalize();

    assert_eq!(store.queue[0].id, "youtube:track:abc");
    assert_eq!(
        store.queue[0].album_id.as_deref(),
        Some("spotify:album:fedcba9876543210fedcba9876543210")
    );
    assert_eq!(store.queue[0].artist_ids, ["youtube:channel:UC1"]);
    assert_eq!(store.playlists[0].id, "local:playlist:1a");
    assert_eq!(store.order, ["local:playlist:1a".to_owned()]);
    assert_eq!(store.rooms[0].id, "local:room:relay|code");
}

#[test]
fn legacy_follows_move_to_the_keyed_shape() {
    let mut store = Store::default();
    store.follows.push(Follow {
        id: "abc".to_owned(),
        name: "Artist".to_owned(),
        kind: FollowKind::Artist,
        ..Default::default()
    });
    store.follows_at.insert("abc".to_owned(), 7);
    store.normalize();

    assert_eq!(store.follows[0].id, "spotify:artist:abc");
    assert_eq!(store.follows[0].name, "Artist");
    assert_eq!(store.follows_at["spotify:artist:abc"], 7);
}

#[test]
fn users_and_channels_keep_their_platform() {
    let mut store = Store::default();
    store.follows.push(Follow {
        id: "bob".to_owned(),
        kind: FollowKind::User,
        ..Default::default()
    });
    store.follows.push(Follow {
        id: "UC1".to_owned(),
        kind: FollowKind::Channel,
        ..Default::default()
    });
    store.normalize();

    assert_eq!(store.follows[0].id, "spotify:user:bob");
    assert_eq!(store.follows[1].id, "youtube:channel:UC1");
}
