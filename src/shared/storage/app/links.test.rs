use std::fs;

use super::{forget, migrate, read, write};
use crate::shared::storage::cache;

#[test]
fn a_pick_reads_back_as_the_app_names_it() {
    write("spotify:track:links-roundtrip", "youtube:track:dQw4w9WgXcQ");

    let path = cache::raw_path("youtube/links/spotify:track:links-roundtrip").unwrap();
    assert_eq!(
        fs::read(&path).unwrap(),
        b"dQw4w9WgXcQ",
        "the file holds the video id alone, as Jota writes it"
    );

    assert_eq!(
        read("spotify:track:links-roundtrip").as_deref(),
        Some("youtube:track:dQw4w9WgXcQ")
    );

    forget("spotify:track:links-roundtrip");
    assert!(read("spotify:track:links-roundtrip").is_none());
}

/// A pick pasted bare lands on the id the app writes, so one song never keeps
/// two entries.
#[test]
fn a_bare_video_lands_on_the_full_id() {
    write("spotify:track:links-bare", "dQw4w9WgXcQ");
    assert_eq!(
        read("spotify:track:links-bare").as_deref(),
        Some("youtube:track:dQw4w9WgXcQ")
    );
}

/// The links an older state carried move to the cache, so the state keeps only
/// the pins.
#[test]
fn the_state_links_move_to_the_cache() {
    migrate(vec![(
        "spotify:track:links-migrated".to_owned(),
        "youtube:track:dQw4w9WgXcQ".to_owned(),
    )]);

    assert_eq!(
        read("spotify:track:links-migrated").as_deref(),
        Some("youtube:track:dQw4w9WgXcQ")
    );
}
