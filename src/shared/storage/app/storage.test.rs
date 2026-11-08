use crate::shared::music::domain::Room;

use crate::shared::storage::domain::*;

use crate::shared::music::domain::{Follow, FollowKind};

#[test]
fn a_state_without_a_browser_keeps_the_session() {
    let store: Store =
        serde_json::from_str(r#"{"youtube_cookies":"SAPISID=abc","youtube_full":true}"#).unwrap();
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
