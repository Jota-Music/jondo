//! `jota.backup.json`, the document a Jota library travels in.
//!
//! Jota keeps every device converging through one JSON file in a folder the
//! user syncs with whatever they already use. The shape is Jota's: the sections
//! are read and written here exactly as it writes them, so the same folder can
//! be shared by both apps.

use std::fs;
use std::path::{Path, PathBuf};

use crate::shared::storage::Store;

use super::*;
use super::{document::*, playlists::*, sections::*};

pub(super) const VERSION: u32 = 1;
pub(super) const DEFAULT_NAME: &str = "New playlist";

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Newer,
    Restore,
}

/// The pass that runs, and why. It travels with the event so the panel only
/// paints a status for the passes the user asked for: the automatic one runs
/// every few seconds and would blink a label on and off.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Job {
    Auto,
    Save,
    Reload,
    Overwrite,
}

impl Job {
    pub fn visible(self) -> bool {
        !matches!(self, Job::Auto)
    }

    pub fn run(self, store: &mut Store) -> Result<Stats, String> {
        match self {
            Job::Auto => auto(store),
            Job::Save => save(store),
            Job::Reload => restore(store),
            Job::Overwrite => overwrite(store),
        }
    }
}

#[derive(Debug, Default)]
pub struct Stats {
    pub bytes: usize,
    pub in_file: usize,
    pub hash: u32,
    pub playlists: usize,
    pub follows: usize,
    pub saved: usize,
    pub order: usize,
    pub settings: usize,
    pub sessions: usize,
    pub modified: bool,
}

impl Stats {
    pub fn changed(&self) -> usize {
        self.playlists + self.follows + self.saved + self.order + self.settings + self.sessions
    }
}

pub use crate::settings::domain::SyncStatus;

/// The automatic pass: it settles with a stat when the folder still holds the
/// copy this device reconciled and nothing is waiting to be written.
pub fn auto(store: &mut Store) -> Result<Stats, String> {
    pass(store, Mode::Newer, false, true)
}

/// The manual save: it always reads and merges, and writes this library now.
pub fn save(store: &mut Store) -> Result<Stats, String> {
    pass(store, Mode::Newer, false, false)
}

/// The emergency exit: it writes this document over one a save cannot read.
pub fn overwrite(store: &mut Store) -> Result<Stats, String> {
    pass(store, Mode::Newer, true, false)
}

/// An explicit import: the file wins over local deletions.
pub fn restore(store: &mut Store) -> Result<Stats, String> {
    pass(store, Mode::Restore, false, false)
}

/// True when a manual save could still change the document: our library moved
/// since the last pass, the location was never reconciled here, or a pass failed.
pub fn pending(store: &Store) -> bool {
    let folder = store.sync_folder.trim();
    if folder.is_empty() {
        return false;
    }

    let status = &store.sync_status;
    status.at == 0
        || status.uri != folder
        || status.stale
        || !status.problem.is_empty()
        || store.sync_pending
}

/// Records that a pass ran without reconciling anything, so the panel's "last
/// save" is this pass and not the last write.
pub fn touch(store: &mut Store) {
    store.sync_status.at = crate::shared::kernel::app::clock::now_ms();
}

/// FNV-1a 32, the fingerprint Jota uses for the document it wrote.
pub fn stamp(text: &str) -> u32 {
    let mut hash: u32 = 0x811c_9dc5;
    for byte in text.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }

    hash
}

pub(super) fn pass(
    store: &mut Store,
    mode: Mode,
    force: bool,
    settle: bool,
) -> Result<Stats, String> {
    let folder = store.sync_folder.trim().to_owned();
    if folder.is_empty() {
        return Err("Set a backup folder first".to_owned());
    }

    let folder = PathBuf::from(folder);
    let outcome = attempt(store, &folder, mode, force, settle);
    if let Err(error) = &outcome {
        problem(store, error);
    }

    outcome
}

pub(super) fn attempt(
    store: &mut Store,
    folder: &Path,
    mode: Mode,
    force: bool,
    settle: bool,
) -> Result<Stats, String> {
    if settle && !force && mode == Mode::Newer && settled(store) {
        touch(store);
        return Ok(Stats::default());
    }

    let (mut document, bytes, found) = read(folder, force)?;
    let import = mode == Mode::Restore || store.sync_import;
    let mut stats = Stats {
        bytes,
        ..Default::default()
    };

    stats.playlists = playlists(&mut document, store, mode, import);
    stats.follows = follows(&mut document, store, mode, import);
    stats.saved = saved(&mut document, store, mode, import);
    stats.order = order(&mut document, store, mode, import);
    stats.settings = settings(&mut document, store, mode, import);
    stats.sessions = sessions(&mut document, store, mode, import);
    store.sync_status.sealed = sealed(&document, &store.session_pass);
    stats.in_file = document
        .playlists
        .as_ref()
        .map_or(0, |part| part.items.len());

    if !found && document.empty() {
        return Err(format!("No backup in {}", folder.display()));
    }

    let (written, hash) = write(folder, &document)?;
    stats.modified = written;
    stats.hash = hash;
    record(store, &stats, true, written, 0);
    store.sync_pending = false;
    Ok(stats)
}

/// Writes the document to an arbitrary file: the manual backup the panel
/// offers, next to the folder autosave keeps.
pub fn export(store: &mut Store, path: &Path) -> Result<Stats, String> {
    let mut document = Document::default();
    let mut stats = Stats {
        playlists: playlists(&mut document, store, Mode::Newer, false),
        ..Default::default()
    };
    stats.follows = follows(&mut document, store, Mode::Newer, false);
    stats.saved = saved(&mut document, store, Mode::Newer, false);
    stats.order = order(&mut document, store, Mode::Newer, false);
    stats.settings = settings(&mut document, store, Mode::Newer, false);
    stats.sessions = sessions(&mut document, store, Mode::Newer, false);

    let text = serde_json::to_string(&document)
        .map_err(|error| format!("backup: cannot encode: {error}"))?;
    fs::write(path, text).map_err(|error| format!("{}: {error}", path.display()))?;
    Ok(stats)
}

/// Applies a file written by `export` (or by Jota). Every record and setting
/// wins here, which is what "import" means: the file is the user's answer.
pub fn import(store: &mut Store, path: &Path) -> Result<Stats, String> {
    let text = fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let mut document = match Document::parse(&text) {
        Ok(document) => document,
        Err(failure) => return Err(failure.to_string()),
    };

    let mut stats = Stats {
        bytes: text.len(),
        ..Default::default()
    };

    stats.playlists = playlists(&mut document, store, Mode::Restore, true);
    stats.follows = follows(&mut document, store, Mode::Restore, true);
    stats.saved = saved(&mut document, store, Mode::Restore, true);
    stats.order = order(&mut document, store, Mode::Restore, true);
    stats.settings = settings(&mut document, store, Mode::Restore, true);
    stats.sessions = sessions(&mut document, store, Mode::Restore, true);
    Ok(stats)
}

/// The settled check of the document path (Jota's `settled` in the page): a
/// provider can answer the stat from its own cache, so a copy is only trusted
/// for the interval the settings promise. `size` is the provider's reported size
/// and is only used when it has no stamp to offer.
pub fn settled_document(store: &Store, last_modified: i64, size: i64) -> bool {
    let uri = store.sync_folder.trim();
    if uri.is_empty() || pending(store) {
        return false;
    }

    let status = &store.sync_status;
    if status.uri != uri || status.stale || !status.problem.is_empty() {
        return false;
    }

    let interval = (store.sync_interval.max(1) as i64) * 60_000;
    if crate::shared::kernel::app::clock::now_ms() - status.read_at > interval {
        return false;
    }

    if last_modified == 0 {
        return status.bytes > 0 && size == status.bytes as i64;
    }

    status.read_stamp > 0 && last_modified == status.read_stamp
}

/// True when the folder still holds the copy this device reconciled and the
/// library has nothing new to write, which makes a pass a stat instead of a read.
pub(crate) fn settled(store: &Store) -> bool {
    if store.sync_pending {
        return false;
    }

    let folder = PathBuf::from(store.sync_folder.trim());
    let Some((mtime, size)) = info(&folder) else {
        return false;
    };

    let status = &store.sync_status;
    mtime == status.mtime && size == status.file_size
}

/// The document's own mtime and size, the fingerprint a settled folder is judged
/// against. Nanoseconds and bytes, like Jota's stat.
pub(super) fn info(folder: &Path) -> Option<(i64, i64)> {
    let metadata = fs::metadata(folder.join(FILE)).ok()?;
    let mtime = metadata
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_nanos() as i64;
    Some((mtime, metadata.len() as i64))
}

pub(super) fn record(store: &mut Store, stats: &Stats, fresh: bool, written: bool, read: i64) {
    let at = crate::shared::kernel::app::clock::now_ms();
    let stale = stats.modified && !fresh;
    let uri = store.sync_folder.trim().to_owned();
    {
        let status = &mut store.sync_status;
        status.at = at;
        status.uri = uri.clone();
        status.read_at = at;
        status.bytes = stats.bytes;
        status.remote = stats.in_file;
        status.imported = stats.changed();
        status.modified = stats.modified;
        status.stale = stale;
        status.read_stamp = read;
        status.problem.clear();
        status.mtime = 0;
        status.file_size = 0;
        if written && stats.modified {
            status.write_at = at;
            status.hash = stats.hash;
        }
    }

    if !uri.is_empty()
        && let Some((mtime, size)) = info(Path::new(&uri))
    {
        store.sync_status.mtime = mtime;
        store.sync_status.file_size = size;
    }
}

pub(super) fn problem(store: &mut Store, error: &str) {
    log::warn!("backup: {error}");
    store.sync_status.at = crate::shared::kernel::app::clock::now_ms();
    store.sync_status.problem = error.to_owned();
}

pub(super) fn mark_stale(store: &mut Store) {
    if store.sync_status.stale {
        return;
    }

    store.sync_status.stale = true;
}

/// The rule the reading side applies to a copy a provider served. Only positive
/// evidence of a stale copy skips the write: bytes that are not ours and a
/// provider stamp that did not move after our last write.
pub(super) fn fresh_copy(store: &Store, data: &str, last_modified: i64) -> bool {
    let status = &store.sync_status;
    if status.uri != store.sync_folder.trim() || status.write_at == 0 {
        return true;
    }

    if stamp(data) == status.hash {
        return true;
    }

    last_modified == 0 || last_modified > status.write_at
}

#[derive(Debug)]
pub(super) enum Failure {
    Version(u32),
    Unreadable(String),
}

impl std::fmt::Display for Failure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Failure::Version(version) => {
                write!(formatter, "backup: version {version} is not supported")
            }
            Failure::Unreadable(error) => write!(formatter, "backup: cannot read ({error})"),
        }
    }
}

pub(super) fn read(folder: &Path, force: bool) -> Result<(Document, usize, bool), String> {
    let canonical = folder.join(FILE);
    if let Ok(text) = fs::read_to_string(&canonical) {
        match Document::parse(&text) {
            Ok(document) => return Ok((document, text.len(), true)),
            Err(Failure::Unreadable(error)) => {
                log::warn!("backup: overwriting {}: {error}", canonical.display());
                return Ok((Document::default(), 0, true));
            }
            Err(Failure::Version(_)) if force => {
                return Ok((Document::default(), 0, true));
            }
            Err(failure) => return Err(failure.to_string()),
        }
    }

    let Ok(entries) = fs::read_dir(folder) else {
        return Ok((Document::default(), 0, false));
    };

    let mut candidates: Vec<(std::time::SystemTime, PathBuf)> = entries
        .flatten()
        .filter(|entry| entry.path().extension().is_some_and(|kind| kind == "json"))
        .filter_map(|entry| Some((entry.metadata().ok()?.modified().ok()?, entry.path())))
        .collect();
    candidates.sort_by_key(|entry| std::cmp::Reverse(entry.0));

    for (_, path) in candidates {
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };

        if let Ok(document) = Document::parse(&text) {
            log::info!("backup: using {}", path.display());
            return Ok((document, text.len(), true));
        }
    }

    Ok((Document::default(), 0, false))
}

pub(super) fn write(folder: &Path, document: &Document) -> Result<(bool, u32), String> {
    fs::create_dir_all(folder).map_err(|error| format!("{}: {error}", folder.display()))?;
    let text = serde_json::to_string(document)
        .map_err(|error| format!("backup: cannot encode: {error}"))?;
    let hash = stamp(&text);
    let path = folder.join(FILE);
    if fs::read_to_string(&path).is_ok_and(|current| current == text) {
        return Ok((false, hash));
    }

    let temp = path.with_extension("tmp");
    fs::write(&temp, &text).map_err(|error| format!("{}: {error}", temp.display()))?;
    fs::rename(&temp, &path).map_err(|error| format!("{}: {error}", path.display()))?;
    Ok((true, hash))
}
