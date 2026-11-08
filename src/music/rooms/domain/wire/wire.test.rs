use super::{ServerMessage, WireSong};
use crate::shared::music::domain::{Song, Source};

#[test]
fn jota_songs_convert_both_ways() {
    let json = r#"{
        "id":"youtube:abc",
        "url":"",
        "name":"Track",
        "duration":120,
        "share":{"id":"abc","url":""},
        "album":{"id":"","title":"Album","url":"","covers":["https://cover"]},
        "artists":[{"id":"","name":"Artist","source":""}],
        "youtubeId":"abc"
    }"#;
    let wire: WireSong = serde_json::from_str(json).expect("jota song");
    let song = Song::from(wire.clone());
    assert_eq!(song.name, "Track");
    assert_eq!(song.artists, ["Artist"]);
    assert_eq!(song.album, "Album");
    assert_eq!(song.cover.as_deref(), Some("https://cover"));
    assert_eq!(song.video(), Some("abc"));
    assert_eq!(song.source, Source::YouTube);

    let back = serde_json::to_value(WireSong::from(&song)).expect("jondo song");
    assert_eq!(back["youtubeId"], "abc");
    assert_eq!(back["album"]["title"], "Album");
    assert_eq!(back["artists"][0]["name"], "Artist");
    assert!(back.get("video_id").is_none());
}

#[test]
fn control_messages_keep_camel_case() {
    let msg: ServerMessage =
        serde_json::from_str(r#"{"t":"control","action":"seek","positionMs":1500}"#)
            .expect("control");
    let ServerMessage::Control(control) = msg else {
        panic!("not a control");
    };

    assert_eq!(control.action, "seek");
    assert_eq!(control.position_ms, 1500.0);
}
