use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use serde::{Deserialize, Serialize};

use crate::shared::music::domain::Song;

use crate::shared::storage::domain::zero;

/// The queue left the state file: it is the biggest thing the app keeps and it
/// moves far less often than the settings around it. It lives in the cache
/// folder next to the state file and is only written when it actually moved.
#[derive(Serialize, Deserialize)]
struct Queue {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    songs: Vec<Song>,
    #[serde(default, skip_serializing_if = "zero")]
    index: usize,
}

/// The last queue written, and where. Two queues with the same fingerprint are
/// the same file, so the state file can be written on every change without
/// dragging the queue along.
static WRITTEN: Mutex<Option<(PathBuf, String)>> = Mutex::new(None);

/// The queue file that belongs to a state file.
fn path(state: &Path) -> Option<PathBuf> {
    Some(state.parent()?.join("cache").join("queue.json"))
}

/// The queue as it was left on disk.
pub(super) fn load(state: &Path) -> Option<(Vec<Song>, usize)> {
    let text = fs::read_to_string(path(state)?).ok()?;
    let queue: Queue = serde_json::from_str(&text).ok()?;
    Some((queue.songs, queue.index))
}

/// Writes the queue, but only when it moved.
pub(super) fn save(state: &Path, songs: &[Song], index: usize) {
    let Some(path) = path(state) else {
        return;
    };

    let fingerprint = crate::shared::storage::cache::fingerprint(&(songs, index));
    {
        let mut written = WRITTEN.lock().unwrap_or_else(PoisonError::into_inner);
        if written
            .as_ref()
            .is_some_and(|(last, hash)| last == &path && *hash == fingerprint)
        {
            return;
        }
        *written = Some((path.clone(), fingerprint));
    }

    if let Some(parent) = path.parent()
        && let Err(error) = fs::create_dir_all(parent)
    {
        log::warn!("cannot create {}: {error}", parent.display());
        return;
    }

    let queue = Queue {
        songs: songs.to_vec(),
        index,
    };
    let Ok(text) = serde_json::to_string(&queue) else {
        log::warn!("cannot encode the queue");
        return;
    };

    let temp = path.with_extension("tmp");
    if let Err(error) = fs::write(&temp, &text).and_then(|()| fs::rename(&temp, &path)) {
        let _ = fs::remove_file(&temp);
        log::warn!("cannot save {}: {error}", path.display());
    }
}

// (the fingerprint is now cache::fingerprint over (songs, index))
