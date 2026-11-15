//! The video a search settled on for a song, one file per song.
//!
//! It is a pick, not a decision: it lives in the cache, addressed by the song
//! id, and holds the video id alone, with no envelope. Losing one costs a
//! search, never a track. Only the pins the user fixed by hand live in the
//! state, so a learned link never lands in the backup as if it were one.

use std::fs;

use crate::shared::music::domain::id;
use crate::shared::storage::cache;

fn name(song: &str) -> String {
    format!("youtube/links/{song}")
}

fn path(song: &str) -> Option<std::path::PathBuf> {
    cache::raw_path(&name(song))
}

/// The video an earlier search settled on, as the app names it.
pub fn read(song: &str) -> Option<String> {
    let video = String::from_utf8(fs::read(path(song)?).ok()?).ok()?;
    let video = id::video(&video);
    (!video.is_empty()).then_some(video)
}

/// Stores the pick. A cache that cannot be written costs one search.
pub fn write(song: &str, video: &str) {
    if song.is_empty() || video.is_empty() {
        return;
    }

    let Some(path) = path(song) else {
        return;
    };

    if let Err(error) = cache::write_at(&path, id::bare(video).as_bytes()) {
        log::debug!("youtube link: {error}");
    }
}

pub fn forget(song: &str) {
    if let Some(path) = path(song) {
        let _ = fs::remove_file(path);
    }
}

/// Moves the links an older state file carried into the cache, so the state
/// only holds pins. Idempotent: a device with no links is a no-op.
pub fn migrate(links: impl IntoIterator<Item = (String, String)>) {
    for (song, video) in links {
        let video = id::video(&video);
        if !song.is_empty() && !video.is_empty() {
            write(&song, &video);
        }
    }
}

#[cfg(test)]
#[path = "links.test.rs"]
mod tests;
