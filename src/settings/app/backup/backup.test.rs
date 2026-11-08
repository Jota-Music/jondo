//! `jota.backup.json`, the document a Jota library travels in.
//!
//! Jota keeps every device converging through one JSON file in a folder the
//! user syncs with whatever they already use. The shape is Jota's: the sections
//! are read and written here exactly as it writes them, so the same folder can
//! be shared by both apps.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use serde_json::Value;

use crate::shared::music::domain::{Song, Source};
use crate::shared::storage::{LocalPlaylist, Store};

use super::*;
use super::{document::*, format::*, playlists::*, sections::*, stage::*, sync::*};

fn playlist(id: &str, name: &str, modified: u64) -> LocalPlaylist {
    LocalPlaylist {
        id: id.to_owned(),
        name: name.to_owned(),
        songs: vec![Song {
            id: "spotify:track:abc".to_owned(),
            name: "Song".to_owned(),
            artists: vec!["Artist".to_owned()],
            artist_ids: vec!["0123456789abcdef0123456789abcdef".to_owned()],
            album: "Album".to_owned(),
            album_id: Some("fedcba9876543210fedcba9876543210".to_owned()),
            cover: Some("https://cover".to_owned()),
            duration: 200,
            source: Source::Spotify,
            ..Default::default()
        }],
        modified,
    }
}

#[test]
fn the_document_matches_the_wire_shape_jota_writes() {
    let mut store = Store::default();
    store.playlists.push(playlist("local:abc", "Mix", 1));
    store
        .follows
        .push(crate::shared::music::domain::Follow::canonical(
            "spotify:artist:1",
        ));
    store.follows_at.insert("artist:1".to_owned(), 5);
    store.order = vec!["local:abc".to_owned()];
    store.order_at = 7;

    let mut document = Document::default();
    playlists(&mut document, &mut store, Mode::Newer, false);
    follows(&mut document, &mut store, Mode::Newer, false);
    order(&mut document, &mut store, Mode::Newer, false);

    let text = serde_json::to_string(&document).unwrap();
    let value: Value = serde_json::from_str(&text).unwrap();

    assert_eq!(value["version"], 1);
    assert_eq!(value["playlists"]["items"][0]["source"], "local");
    assert_eq!(value["playlists"]["items"][0]["modifiedAt"], 1000);
    assert_eq!(
        value["playlists"]["items"][0]["songs"][0]["id"],
        "spotify:track:abc"
    );
    assert_eq!(
        value["playlists"]["songs"]["spotify:track:abc"]["name"],
        "Song"
    );
    let artist = &value["playlists"]["songs"]["spotify:track:abc"]["artists"][0]["id"];
    assert_eq!(
        value["playlists"]["artists"][artist.as_str().unwrap()]["name"],
        "Artist"
    );
    assert!(value["playlists"]["songs"]["spotify:track:abc"]["album"]["id"].is_string());
    assert_eq!(value["follows"]["items"][0]["at"], 5);
    assert_eq!(value["order"][0]["scope"], "library");
    assert_eq!(value["order"][0]["ids"][0], "local:abc");
    assert!(value.get("settings").is_none());
}

#[test]
fn only_user_picks_travel_in_the_document() {
    let mut store = Store::default();
    store.link("spotify:track:auto", "autoVideo01");
    store.pick("spotify:track:picked", "pickedVideo");

    let mut document = Document::default();
    playlists(&mut document, &mut store, Mode::Newer, false);

    let songs = &document.playlists.as_ref().unwrap().songs;
    assert_eq!(songs["spotify:track:picked"].youtube_id, "pickedVideo");
    assert!(!songs.contains_key("spotify:track:auto"));
}

#[test]
fn a_document_pick_does_not_replace_a_local_pick() {
    let text = r#"{"version":1,"playlists":{"songs":{
        "spotify:track:a":{"youtubeId":"theirs"},
        "spotify:track:b":{"youtubeId":"theirs2"}
    }}}"#;
    let mut store = Store::default();
    store.pick("spotify:track:a", "mine");
    let mut document = Document::parse(text).unwrap();
    playlists(&mut document, &mut store, Mode::Newer, true);

    assert_eq!(store.youtube_links["spotify:track:a"], "mine");
    assert_eq!(store.youtube_links["spotify:track:b"], "theirs2");
    assert!(store.youtube_picks.contains("spotify:track:b"));
}

#[test]
fn a_document_written_elsewhere_round_trips_through_the_store() {
    let text = r#"{
        "version": 1,
        "playlists": {
            "items": [
                {"id": "local:one", "source": "local", "name": "From Jota",
                 "modifiedAt": 2000, "songs": [{"id": "youtube:xyz"}]}
            ],
            "songs": {"youtube:xyz": {"name": "Video", "duration": 61, "youtubeId": "xyz"}},
            "deleted": {"local:gone": 1000}
        },
        "order": [{"scope": "library", "ids": ["local:one"], "at": 2000}],
        "sessions": {"spotify": {"value": "JOTAENC1:blob", "at": 5}}
    }"#;

    let mut store = Store::default();
    let mut document = Document::parse(text).unwrap();
    playlists(&mut document, &mut store, Mode::Newer, true);
    order(&mut document, &mut store, Mode::Newer, true);

    assert_eq!(store.playlists.len(), 1);
    assert_eq!(store.playlists[0].name, "From Jota");
    assert_eq!(store.playlists[0].modified, 2);
    assert_eq!(store.playlists[0].songs[0].video_id.as_deref(), Some("xyz"));
    assert_eq!(store.playlist_graves["local:gone"], 1000);
    assert_eq!(store.order, vec!["local:one".to_owned()]);

    let rendered = serde_json::to_string(&document).unwrap();
    assert!(rendered.contains("\"sessions\":{\"spotify\""));
}

#[test]
fn a_tombstone_keeps_a_playlist_from_coming_back() {
    let mut store = Store::default();
    store.playlist_graves.insert("local:dead".to_owned(), 5000);
    let text = r#"{"version":1,"playlists":{"items":[
        {"id":"local:dead","source":"local","name":"Back","modifiedAt":4000,"songs":[]}
    ]}}"#;
    let mut document = Document::parse(text).unwrap();
    playlists(&mut document, &mut store, Mode::Newer, true);
    assert!(store.playlists.is_empty());
}

#[test]
fn newer_copy_wins_and_metadata_is_additive() {
    let mut store = Store::default();
    store.playlists.push(playlist("local:abc", "Mine", 1));
    let text = r#"{"version":1,"playlists":{"items":[
        {"id":"local:abc","source":"local","name":"Theirs","modifiedAt":9000,
         "songs":[{"id":"spotify:track:abc"},{"id":"spotify:track:def"}]}
      ],"songs":{"spotify:track:def":{"name":"Only there"}}}}"#;
    let mut document = Document::parse(text).unwrap();
    playlists(&mut document, &mut store, Mode::Newer, true);

    let merged = &store.playlists[0];
    assert_eq!(merged.name, "Theirs");
    assert_eq!(merged.songs.len(), 2);
    assert_eq!(merged.songs[1].name, "Only there");
    assert_eq!(merged.songs[0].name, "Song");
}

#[test]
fn a_pass_writes_the_document_and_reads_it_back() {
    let folder = std::env::temp_dir().join(format!("jondo-backup-{}", std::process::id()));
    let _ = fs::remove_dir_all(&folder);
    let folder_text = folder.display().to_string();

    let mut store = Store::default();
    store.playlists.push(playlist("local:abc", "Mix", 1));
    store.sync_folder = folder_text.clone();

    let stats = save(&mut store).unwrap();
    assert!(stats.modified);

    let text = fs::read_to_string(folder.join(FILE)).unwrap();
    assert!(
        text.starts_with(r#"{"version":1,"playlists":{"items":"#),
        "{text}"
    );

    let again = save(&mut store).unwrap();
    assert!(!again.modified, "an unchanged document is not rewritten");

    let mut reader = Store {
        sync_folder: folder_text.clone(),

        ..Default::default()
    };
    let stats = save(&mut reader).unwrap();
    assert_eq!(stats.playlists, 1);
    assert_eq!(reader.playlists[0].name, "Mix");
    assert_eq!(reader.playlists[0].songs[0].name, "Song");

    let _ = fs::remove_dir_all(&folder);
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("jondo-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn the_freshness_rule_only_trusts_positive_evidence() {
    let mut store = Store {
        sync_folder: "/sync".to_owned(),
        ..Default::default()
    };
    let data = r#"{"version":1}"#;

    assert!(fresh_copy(&store, data, 0), "no baseline is trusted");

    store.sync_status.uri = "/sync".to_owned();
    store.sync_status.write_at = 2000;
    store.sync_status.hash = stamp(data);
    assert!(fresh_copy(&store, data, 0), "our own bytes are ours");

    let other = r#"{"version":1,"order":[]}"#;
    assert!(
        !fresh_copy(&store, other, 1999),
        "an older copy that is not ours is stale"
    );
    assert!(fresh_copy(&store, other, 2001), "a newer copy wins");
    assert!(fresh_copy(&store, other, 0), "no stamp carries no proof");
}

#[test]
fn a_settled_folder_is_a_stat_and_a_change_is_not() {
    let folder = scratch("backup-settled");
    let folder_text = folder.display().to_string();
    let mut store = Store {
        sync_folder: folder_text,
        ..Default::default()
    };
    store.playlists.push(playlist("local:abc", "Mix", 1));

    let stats = save(&mut store).unwrap();
    assert!(stats.modified);
    assert!(settled(&store), "the pass just recorded the file");
    assert!(!pending(&store));

    let quiet = auto(&mut store).unwrap();
    assert!(!quiet.modified);
    assert_eq!(quiet.bytes, 0, "a settled pass reads nothing");

    store.sync_pending = true;
    assert!(!settled(&store));
    store.sync_pending = false;

    std::thread::sleep(std::time::Duration::from_millis(20));
    fs::write(folder.join(FILE), r#"{"version":1}"#).unwrap();
    assert!(!settled(&store), "another device's write is a change");

    let _ = fs::remove_dir_all(&folder);
}

#[test]
fn the_document_path_stages_and_moves_the_baseline_on_commit() {
    let dir = scratch("backup-stage");
    fs::write(
        dir.join("incoming.json"),
        r#"{"version":1,"playlists":{"items":[
            {"id":"local:x","source":"local","name":"Staged","modifiedAt":10,"songs":[]}
        ]}}"#,
    )
    .unwrap();

    let mut store = Store {
        sync_folder: "content://doc".to_owned(),

        ..Default::default()
    };

    let (stats, write) = stage(&mut store, &dir, 0, false).unwrap();
    assert!(write, "a foreign copy is written back");
    assert_eq!(stats.playlists, 1);
    assert_eq!(store.playlists.len(), 1);
    assert!(dir.join("outgoing.json").exists());
    assert!(!dir.join("incoming.json").exists());

    committed(&mut store, &dir);
    assert_eq!(store.sync_status.write_at, store.sync_status.at);
    assert!(!store.sync_pending);
    assert!(!dir.join("outgoing.json").exists());

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_copy_the_provider_served_from_its_cache_is_not_written_over() {
    let dir = scratch("backup-stale");
    fs::write(
        dir.join("incoming.json"),
        r#"{"version":1,"playlists":{"items":[]}}"#,
    )
    .unwrap();

    let mut store = Store::default();
    store.playlists.push(playlist("local:abc", "Mine", 1));
    store.sync_folder = "content://doc".to_owned();
    store.sync_status.uri = "content://doc".to_owned();
    store.sync_status.write_at = 5000;
    store.sync_status.hash = stamp("our own copy");

    let (_, write) = stage(&mut store, &dir, 4000, false).unwrap();
    assert!(!write, "a copy older than our write is not trusted");
    assert!(store.sync_status.stale);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_cached_stamp_is_only_trusted_for_the_interval() {
    let mut store = Store {
        sync_folder: "content://doc".to_owned(),
        ..Default::default()
    };
    store.sync_status.uri = "content://doc".to_owned();
    store.sync_status.at = crate::shared::kernel::app::clock::now_ms();
    store.sync_status.read_at = store.sync_status.at;
    store.sync_status.read_stamp = 100;
    assert!(settled_document(&store, 100, 0));
    assert!(!settled_document(&store, 101, 0), "another copy is a pass");

    store.sync_status.read_at = crate::shared::kernel::app::clock::now_ms() - 16 * 60_000;
    assert!(
        !settled_document(&store, 100, 0),
        "past the interval the document is read again"
    );

    store.sync_status.read_at = crate::shared::kernel::app::clock::now_ms();
    store.sync_status.read_stamp = 0;
    store.sync_status.bytes = 400;
    assert!(
        settled_document(&store, 0, 400),
        "no stamp falls back to size"
    );
    assert!(!settled_document(&store, 0, 399));
}

#[test]
fn export_and_import_round_trip_a_library() {
    let mut store = Store::default();
    store.playlists.push(playlist("local:abc", "Mix", 1));
    store.order = vec!["local:abc".to_owned()];
    store.order_at = 5;

    let dir = std::env::temp_dir().join(format!(
        "jondo-export-{}",
        crate::shared::kernel::app::clock::now_ms()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(FILE);
    export(&mut store, &path).unwrap();

    let mut restored = Store::default();
    import(&mut restored, &path).unwrap();
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(restored.playlists.len(), 1);
    assert_eq!(restored.playlists[0].name, "Mix");
    assert_eq!(restored.order, vec!["local:abc".to_owned()]);
}

#[test]
fn a_session_this_device_cannot_open_is_reported() {
    let mut store = Store {
        session_pass: "secret".to_owned(),
        ..Default::default()
    };
    store.youtube_cookies = "SAPISID=abc; __SSID=def".to_owned();
    store.youtube_at = 10;

    let mut document = Document::default();
    sessions(&mut document, &mut store, Mode::Newer, false);

    assert!(!sealed(&document, "secret"));
    assert!(sealed(&document, "other"));
    assert!(sealed(&document, ""));
}

#[test]
fn sessions_only_travel_sealed_and_open_with_the_pass() {
    let mut store = Store {
        session_pass: "secret".to_owned(),
        ..Default::default()
    };
    store.youtube_cookies = "SAPISID=abc; __SSID=def".to_owned();
    store.youtube_at = 10;

    let mut document = Document::default();
    sessions(&mut document, &mut store, Mode::Newer, false);
    let blob = document.sessions.as_ref().unwrap()["youtube"]
        .value
        .as_str()
        .unwrap()
        .to_owned();
    assert!(crate::shared::crypto::session::sealed(&blob));
    assert!(!blob.contains("SAPISID"));

    let mut other = Store {
        session_pass: "secret".to_owned(),

        ..Default::default()
    };
    sessions(&mut document, &mut other, Mode::Newer, true);
    assert_eq!(other.youtube_cookies, store.youtube_cookies);

    let mut wrong = Store {
        session_pass: "other".to_owned(),

        ..Default::default()
    };
    sessions(&mut document, &mut wrong, Mode::Newer, true);
    assert!(wrong.youtube_cookies.is_empty());
}

#[test]
fn a_newer_remote_session_stays() {
    let pass = "secret";
    let mut document = Document::default();
    let mut carried = BTreeMap::new();
    carried.insert(
        "youtube".to_owned(),
        Setting {
            value: serde_json::json!(
                crate::shared::crypto::session::encrypt(pass, "SAPISID=remote").unwrap()
            ),
            at: 20,
        },
    );
    document.sessions = Some(carried);

    let mut store = Store {
        session_pass: pass.to_owned(),

        ..Default::default()
    };
    store.youtube_cookies = "SAPISID=local".to_owned();
    store.youtube_at = 10;
    sessions(&mut document, &mut store, Mode::Newer, false);

    let blob = document.sessions.as_ref().unwrap()["youtube"]
        .value
        .as_str()
        .unwrap();
    assert_eq!(
        crate::shared::crypto::session::decrypt(pass, blob).unwrap(),
        "SAPISID=remote"
    );
}

#[test]
fn an_unknown_version_is_refused() {
    assert!(Document::parse(r#"{"version":2}"#).is_err());
}

#[test]
fn ids_from_another_instance_map_to_one_local_record() {
    assert_eq!(stored_id("local:abc"), "local:abc");
    assert_eq!(stored_id("abc"), stored_id("abc"));
    assert!(stored_id("abc").starts_with("local:"));
    assert_ne!(stored_id("abc"), stored_id("abd"));
}
