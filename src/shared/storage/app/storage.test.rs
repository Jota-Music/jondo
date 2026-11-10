use std::fs;

use crate::shared::music::domain::Room;

use crate::shared::storage::domain::*;

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
fn a_default_store_writes_nothing() {
    let text = serde_json::to_string(&Snapshot::from(&Store::default())).unwrap();
    assert_eq!(text, "{}");
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
    store
        .youtube_links
        .insert("spotify:track:one".to_owned(), "video".to_owned());
    store.youtube_picks.insert("spotify:track:one".to_owned());

    let text = serde_json::to_string(&Snapshot::from(&store)).unwrap();
    let back = Store::from(serde_json::from_str::<Snapshot>(&text).unwrap());

    assert_eq!(store, back);
    assert!(
        text.contains(r#""youtube":{"cookies":"SAPISID=abc""#),
        "{text}"
    );
}

#[test]
fn a_state_without_a_browser_keeps_the_session() {
    let store = Store::from(
        serde_json::from_str::<Snapshot>(r#"{"youtube":{"cookies":"SAPISID=abc","full":true}}"#)
            .unwrap(),
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
    assert_eq!(store.youtube_links["spotify:track:abc"], "dQw4w9WgXcQ");
    assert!(store.pick("spotify:track:abc", "dQw4w9WgXcQ"));
    assert!(store.youtube_picks.contains("spotify:track:abc"));
    assert!(!store.link("spotify:track:abc", "other"));
    assert_eq!(store.youtube_links["spotify:track:abc"], "dQw4w9WgXcQ");
    assert!(store.link("spotify:track:abc", ""));
    assert!(store.youtube_links.is_empty());
    assert!(store.youtube_picks.is_empty());
    assert!(!store.pick("youtube:dQw4w9WgXcQ", "dQw4w9WgXcQ"));
    assert!(store.youtube_picks.is_empty());
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

    assert_eq!(store.follows[0].id, "artist:abc");
    assert_eq!(store.follows[0].name, "Artist");
    assert_eq!(store.follows_at["artist:abc"], 7);
}

#[test]
fn users_stay_bare_and_channels_keep_the_youtube_prefix() {
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

    assert_eq!(store.follows[0].id, "bob");
    assert_eq!(store.follows[1].id, "youtube:UC1");
}
