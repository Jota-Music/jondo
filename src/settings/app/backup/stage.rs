//! `jota.backup.json`, the document a Jota library travels in.
//!
//! Jota keeps every device converging through one JSON file in a folder the
//! user syncs with whatever they already use. The shape is Jota's: the sections
//! are read and written here exactly as it writes them, so the same folder can
//! be shared by both apps.

use std::fs;
use std::path::{Path, PathBuf};

use crate::shared::storage::Store;

use super::{document::*, playlists::*, sections::*, sync::*};

/// The dir the native side stages the document in on Android, next to the app's
/// own files (`filesDir/sync`). Desktop has none: it reads the folder directly.
#[cfg(target_os = "android")]
pub fn stage_dir() -> Option<PathBuf> {
    Some(crate::shared::platform::android::data_dir()?.join("sync"))
}

#[cfg(not(target_os = "android"))]
pub fn stage_dir() -> Option<PathBuf> {
    None
}

/// The explicit import on Android: the platform staged the provider's copy as
/// `incoming.json`, so the file wins over local deletions and nothing is written
/// back (a reload is the read direction).
pub fn restore_staged(store: &mut Store, dir: &Path, sync: bool) -> Result<Stats, String> {
    let incoming = dir.join(INCOMING);
    let data = fs::read_to_string(&incoming)
        .map_err(|error| format!("{}: {error}", incoming.display()))?;
    let _ = fs::remove_file(&incoming);

    let mut document = match Document::parse(&data) {
        Ok(document) => document,
        Err(failure) => {
            let message = failure.to_string();
            problem(store, &message);
            return Err(message);
        }
    };

    let mut stats = Stats {
        bytes: data.len(),
        ..Default::default()
    };

    stats.playlists = playlists(&mut document, store, Mode::Restore, true);
    stats.follows = follows(&mut document, store, Mode::Restore, true);
    stats.saved = saved(&mut document, store, Mode::Restore, true);
    stats.order = order(&mut document, store, Mode::Restore, true);
    stats.settings = settings(&mut document, store, Mode::Restore, true);
    stats.sessions = sessions(&mut document, store, Mode::Restore, true);
    store.sync_status.sealed = sealed(&document, &store.session_pass);
    stats.in_file = document
        .playlists
        .as_ref()
        .map_or(0, |part| part.items.len());
    if sync {
        record(store, &stats, true, false, 0);
    }

    Ok(stats)
}

/// The document path: reconcile the copy a platform staged as `incoming.json` in
/// `dir` and leave the answer in `outgoing.json` when it has to be written back.
/// Android reaches its folder through SAF, so the provider's bytes are copied
/// into that file by the native side and never cross as a string.
pub fn stage(
    store: &mut Store,
    dir: &Path,
    last_modified: i64,
    trust: bool,
) -> Result<(Stats, bool), String> {
    let incoming = dir.join(INCOMING);
    let data = fs::read_to_string(&incoming).map_err(|error| {
        let message = format!("{}: {error}", incoming.display());
        problem(store, &message);
        message
    })?;

    let outcome = import_document(store, dir, &data, last_modified, trust);
    let (stats, write) = match outcome {
        Ok(answer) => answer,
        Err(error) => {
            let _ = fs::remove_file(&incoming);
            problem(store, &error);
            return Err(error);
        }
    };

    let _ = fs::remove_file(&incoming);
    if !write {
        return Ok((stats, false));
    }

    let outgoing = dir.join(OUTGOING);
    if let Err(error) = fs::copy(dir.join(MERGED), &outgoing) {
        let message = format!("{}: {error}", outgoing.display());
        problem(store, &message);
        return Err(message);
    }

    Ok((stats, true))
}

/// Records a failure the platform reported, so the panel says why the document
/// stopped being written instead of showing the previous pass as the last one.
pub fn failed(store: &mut Store, reason: &str) {
    if reason.trim().is_empty() {
        return;
    }

    problem(store, reason);
}

const INCOMING: &str = "incoming.json";
const OUTGOING: &str = "outgoing.json";
const MERGED: &str = "merged.json";

/// Merges a staged document and answers whether it has to be written back. The
/// merged text is left in `merged.json` for the commit to pick up.
pub(crate) fn import_document(
    store: &mut Store,
    dir: &Path,
    data: &str,
    last_modified: i64,
    trust: bool,
) -> Result<(Stats, bool), String> {
    let fresh = trust || fresh_copy(store, data, last_modified);
    let import = store.sync_import;

    let mut document = match Document::parse(data) {
        Ok(document) => document,
        Err(Failure::Unreadable(error)) => {
            log::warn!("backup: recovering the document: {error}");
            Document::default()
        }
        Err(failure) => return Err(failure.to_string()),
    };

    let mut stats = Stats {
        bytes: data.len(),
        ..Default::default()
    };

    stats.playlists = playlists(&mut document, store, Mode::Newer, import);
    stats.follows = follows(&mut document, store, Mode::Newer, import);
    stats.saved = saved(&mut document, store, Mode::Newer, import);
    stats.order = order(&mut document, store, Mode::Newer, import);
    stats.settings = settings(&mut document, store, Mode::Newer, import);
    stats.sessions = sessions(&mut document, store, Mode::Newer, import);
    store.sync_status.sealed = sealed(&document, &store.session_pass);
    stats.in_file = document
        .playlists
        .as_ref()
        .map_or(0, |part| part.items.len());

    let merged = serde_json::to_string(&document)
        .map_err(|error| format!("backup: cannot encode: {error}"))?;
    let differs = merged != data;
    stats.modified = fresh && differs;
    stats.hash = stamp(&merged);
    record(store, &stats, fresh, false, last_modified);

    if !stats.modified {
        store.sync_pending = false;
    }

    if !fresh && differs {
        mark_stale(store);
    }

    let write = fresh && differs;
    if write {
        let merged_path = dir.join(MERGED);
        fs::write(&merged_path, merged.as_bytes())
            .map_err(|error| format!("{}: {error}", merged_path.display()))?;
    }

    Ok((stats, write))
}

/// The platform confirming the merged copy reached the file, which is the only
/// moment the write baseline may move.
pub fn committed(store: &mut Store, dir: &Path) {
    let outgoing = dir.join(OUTGOING);
    let Ok(data) = fs::read_to_string(&outgoing) else {
        return;
    };

    let _ = fs::remove_file(&outgoing);
    let at = crate::shared::kernel::app::clock::now_ms();
    let status = &mut store.sync_status;
    status.at = at;
    status.write_at = at;
    status.hash = stamp(&data);
    status.stale = false;
    status.problem.clear();
    store.sync_pending = false;
}
