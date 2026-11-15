use std::fs;
use std::path::Path;
use std::thread::sleep;
use std::time::{Duration, SystemTime};

use super::*;

fn set_modified(path: &Path, at: SystemTime) {
    let file = fs::File::options().write(true).open(path).unwrap();
    file.set_times(fs::FileTimes::new().set_modified(at))
        .unwrap();
}

/// A raw entry owns its name: no envelope, no suffix. It is how a pick under
/// `youtube/links` is stored, so it reads back as the bytes that were written.
#[test]
fn a_raw_entry_owns_its_name() {
    let name = "youtube/links/spotify:track:abc";
    let entry = raw_path(name).unwrap();
    write_at(&entry, b"dQw4w9WgXcQ").unwrap();

    assert_eq!(entry.extension(), None, "a raw entry carries no suffix");
    let file = entry.file_name().unwrap().to_str().unwrap();
    assert!(file.starts_with("spotify_track_abc-"), "{file}");
    assert_eq!(fs::read(&entry).unwrap(), b"dQw4w9WgXcQ");

    fs::remove_file(&entry).unwrap();
    assert!(!entry.exists());
}

/// A store that owns its own shape keeps the `.json` name and no envelope.
#[test]
fn a_raw_entity_keeps_its_shape() {
    write_raw("youtube/session", br#"{"client":"IOS"}"#).unwrap();

    let entry = path("youtube/session").unwrap();
    assert_eq!(
        entry.extension().and_then(|value| value.to_str()),
        Some("json")
    );
    assert_eq!(read_raw("youtube/session").unwrap(), br#"{"client":"IOS"}"#);
    assert!(
        get::<serde_json::Value>("youtube/session").is_none(),
        "raw bytes are not an envelope"
    );
}

/// The envelope carries the moment it was written, so a caller can skip a check
/// the value already passed.
#[test]
fn an_entry_carries_when_it_was_saved() {
    put("spotify/tracks/read", &"a song", None).unwrap();

    let (value, saved) = get_with_meta::<String>("spotify/tracks/read").unwrap();
    assert_eq!(value, "a song");
    assert!(now_ms().saturating_sub(saved) < 1000, "saved {saved}");
}

/// An entry past its expiry is not there: the source answers again.
#[test]
fn an_expired_entry_is_gone() {
    put(
        "youtube/streams/expired",
        &"a url",
        Some(Duration::from_millis(10)),
    )
    .unwrap();
    sleep(Duration::from_millis(20));

    assert!(get::<String>("youtube/streams/expired").is_none());
}

/// The sweep collects the entries whose life is over without reading them: one
/// that old cannot be alive any more, and a fresh one is left alone.
#[test]
fn the_sweep_drops_only_what_is_old() {
    put("youtube/streams/sweep-alive", &"alive", None).unwrap();

    let dead = path("youtube/streams/sweep-dead").unwrap();
    fs::write(&dead, "{}").unwrap();
    set_modified(&dead, SystemTime::now() - Duration::from_secs(48 * 60 * 60));

    assert_eq!(
        sweep("youtube/streams", Duration::from_secs(24 * 60 * 60)),
        1
    );
    assert!(!dead.exists());
    assert!(path("youtube/streams/sweep-alive").unwrap().exists());
}

/// A write that died between the temporary file and the rename leaves one
/// behind, and nothing else would ever collect it. A write this run owns stays.
#[test]
fn a_crashed_write_is_swept_and_a_live_one_is_left() {
    sweep_leftovers();
    sleep(Duration::from_millis(20));

    let folder = cache_dir().unwrap().join("test").join("leftovers");
    fs::create_dir_all(&folder).unwrap();

    let crashed = folder.join("crashed.json.tmp");
    fs::write(&crashed, "{half").unwrap();
    set_modified(&crashed, SystemTime::now() - Duration::from_secs(60));

    let live = folder.join("live.json.tmp");
    fs::write(&live, "half").unwrap();

    sweep_leftovers();

    assert!(!crashed.exists(), "a write a crash cut short is collected");
    assert!(live.exists(), "a write in flight keeps its temporary file");
}

/// A folder is kept under its ceiling by dropping the oldest files first.
#[test]
fn the_trim_keeps_the_newest() {
    let folder = "test/trim";
    let dir = cache_dir().unwrap().join(folder);
    fs::create_dir_all(&dir).unwrap();

    for (index, name) in ["a.jpg", "b.jpg", "c.jpg"].iter().enumerate() {
        let path = dir.join(name);
        fs::write(&path, b"x").unwrap();
        let at = SystemTime::now() - Duration::from_secs(300 - index as u64 * 100);
        set_modified(&path, at);
    }

    trim(folder, Some("jpg"), 2);

    assert!(!dir.join("a.jpg").exists(), "the oldest one goes");
    assert!(dir.join("b.jpg").exists());
    assert!(dir.join("c.jpg").exists());
}
