//! The cache: what the app can fetch again, one file per entity.
//!
//! Jota keeps the same shape on disk: a file per name under `cache/`, with a
//! small envelope holding when it was saved, when it expires and a fingerprint
//! of the value. The helpers here read, write, walk and trim those files, so
//! the features keep their own shape where it matters (a cover is a JPEG, not
//! a JSON envelope) and share the rest.

pub mod wire;

use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// The ceiling of a directory that grows with what the user opens.
pub const LIMIT: usize = 8000;

/// The ceiling of the playlist folder.
pub const PLAYLIST_LIMIT: usize = 48;

/// The ceiling of the cover folder. Each file is one image's bytes, so it is
/// counted smaller than the text entries: a phone should not spend a gigabyte
/// keeping art it can simply fetch again.
const COVER_LIMIT: usize = 400;

/// The ceiling of each folder that grows with what the user opens. A folder
/// that is not here is never trimmed: the picks under `youtube/links` are a few
/// bytes each, and counting them would cost more than the space. An entry that
/// is a single file (the session, the account's library) has nothing to trim.
const LIMITS: &[(&str, &str, usize)] = &[
    ("spotify/playlists", "json", PLAYLIST_LIMIT),
    ("spotify/tracks", "json", LIMIT),
    ("youtube/streams", "json", LIMIT),
    ("youtube/tracks", "json", LIMIT),
    ("covers", "json", COVER_LIMIT),
];

/// The folders whose entries die with the URL they carry. A file older than
/// this cannot still be alive, so the sweep drops it without reading it.
const SWEEPS: &[(&str, Duration)] = &[("youtube/streams", Duration::from_secs(24 * 60 * 60))];

/// When the process began. A temporary file older than it belongs to a write a
/// crash cut short, and nothing else would ever collect it.
static STARTED: OnceLock<SystemTime> = OnceLock::new();

/// The scratch folder the tests share, so a run never reads or writes the real
/// cache. Pinned once per process: every test adds its own names to it.
#[cfg(test)]
pub(crate) fn test_dir() -> PathBuf {
    static SCRATCH: OnceLock<PathBuf> = OnceLock::new();
    SCRATCH
        .get_or_init(|| {
            let dir = std::env::temp_dir().join(format!("jondo-cache-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            dir
        })
        .clone()
}

/// The folder everything cached lives in.
pub fn cache_dir() -> Option<PathBuf> {
    #[cfg(test)]
    let root = Some(test_dir().join("cache"));

    #[cfg(not(test))]
    let root = crate::shared::storage::dir().map(|dir| dir.join("cache"));

    root
}

/// Keeps every folder under its ceiling, drops the URL entries that outlived
/// their link and clears the leftovers a crash wrote. Call once at startup.
pub fn start() {
    sweep_leftovers();

    for (folder, age) in SWEEPS {
        sweep(folder, *age);
    }

    for (folder, ext, limit) in LIMITS {
        trim(folder, Some(ext), *limit);
    }
}

/// The envelope every cached entity shares.
#[derive(Clone, Serialize, Deserialize)]
pub struct Entry<T> {
    /// The name the entity was stored under.
    pub name: String,
    /// When it was saved, in milliseconds since the epoch.
    pub saved: i64,
    /// When it stops being worth something, in milliseconds. Zero means never.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub expire: i64,
    /// A fingerprint of the value, so a refresh can tell whether it changed.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub fingerprint: String,
    /// The value itself.
    pub value: T,
}

fn is_zero(value: &i64) -> bool {
    *value == 0
}

/// Now, in milliseconds since the epoch.
pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis() as i64)
        .unwrap_or_default()
}

/// Turns one part of a name into a file name. Only letters, digits, dashes and
/// underscores survive; the rest becomes an underscore, the name is truncated
/// to 80 characters and four bytes of its sha256 are appended, so two names
/// that differ only in punctuation never share a file.
pub fn safe_part(part: &str) -> String {
    let mut safe: String = part
        .chars()
        .map(|character| match character {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => character,
            _ => '_',
        })
        .collect();

    if safe.len() > 80 {
        safe.truncate(80);
    }

    let sum = Sha256::digest(part.as_bytes());
    let suffix: String = sum[..4].iter().map(|byte| format!("{byte:02x}")).collect();

    format!("{safe}-{suffix}")
}

/// The path a cached entity lives at. The last part of the name is sanitized;
/// the earlier parts become folders.
pub fn path(name: &str) -> Option<PathBuf> {
    location(name, ".json")
}

/// The path of an entry that owns its own shape and does not want the `.json`
/// suffix: the picks under `youtube/links` are a video id, not an envelope.
pub fn raw_path(name: &str) -> Option<PathBuf> {
    location(name, "")
}

fn location(name: &str, suffix: &str) -> Option<PathBuf> {
    let base = cache_dir()?;
    let mut parts: Vec<&str> = name.split('/').collect();
    if parts
        .iter()
        .any(|part| part.is_empty() || *part == "." || *part == "..")
    {
        return None;
    }

    let last = parts.len() - 1;
    let file = format!("{}{suffix}", safe_part(parts[last]));
    parts[last] = &file;

    Some(parts.iter().fold(base, |path, part| path.join(part)))
}

/// The sha256 of a value, as hex. Two values with the same fingerprint are the
/// same file contents.
pub fn fingerprint<T: Serialize>(value: &T) -> String {
    let data = serde_json::to_vec(value).unwrap_or_default();
    let sum = Sha256::digest(&data);
    sum.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Reads a cached entity. An entry past its expiry is dropped, so a stale value
/// never surfaces.
pub fn get<T: for<'de> Deserialize<'de>>(name: &str) -> Option<T> {
    get_with_meta(name).map(|(value, _)| value)
}

/// `get` plus the moment the entry was written. A caller that can tell whether
/// the value was proven recently (a stream's URL was just probed) skips a check
/// without an extra read: the envelope is already here.
pub fn get_with_meta<T: for<'de> Deserialize<'de>>(name: &str) -> Option<(T, i64)> {
    let text = fs::read_to_string(path(name)?).ok()?;
    let entry: Entry<T> = serde_json::from_str(&text).ok()?;

    if entry.expire != 0 && entry.expire <= now_ms() {
        let _ = forget(name);
        return None;
    }

    Some((entry.value, entry.saved))
}

/// Stores a value. The ttl, when given, is how long it stays worth something.
pub fn put<T: Serialize>(name: &str, value: &T, ttl: Option<Duration>) -> Result<(), String> {
    let entry = Entry {
        name: name.to_owned(),
        saved: now_ms(),
        expire: ttl
            .map(|duration| now_ms() + duration.as_millis() as i64)
            .unwrap_or(0),
        fingerprint: fingerprint(value),
        value,
    };

    let path = path(name).ok_or("no cache folder")?;
    let text = serde_json::to_string(&entry).map_err(|error| error.to_string())?;
    write(&path, text.as_bytes())
}

/// Removes a cached entity. A missing file is not an error.
pub fn forget(name: &str) -> Result<(), String> {
    let path = path(name).ok_or("no cache folder")?;

    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}

/// The raw bytes of an entry that owns its own shape: the file keeps the
/// `.json` suffix and carries no envelope.
pub fn read_raw(name: &str) -> Option<Vec<u8>> {
    fs::read(path(name)?).ok()
}

/// Stores raw bytes under a name, with no envelope.
pub fn write_raw(name: &str, data: &[u8]) -> Result<(), String> {
    let path = path(name).ok_or("no cache folder")?;
    write(&path, data)
}

/// Stores raw bytes at a path the caller built, for an entry that has no
/// suffix of its own (a pick under `youtube/links` is a video id).
pub fn write_at(path: &Path, data: &[u8]) -> Result<(), String> {
    write(path, data)
}

/// Replaces a file through a temporary one, so a reader never sees half of it.
fn write(path: &Path, data: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent()
        && let Err(error) = fs::create_dir_all(parent)
    {
        return Err(format!("cannot create {}: {error}", parent.display()));
    }

    let temp = path.with_extension("tmp");
    if let Err(error) = fs::write(&temp, data) {
        let _ = fs::remove_file(&temp);
        return Err(format!("cannot save {}: {error}", path.display()));
    }

    if let Err(error) = fs::rename(&temp, path) {
        let _ = fs::remove_file(&temp);
        return Err(format!("cannot save {}: {error}", path.display()));
    }

    Ok(())
}

/// Drops the entries of a folder older than the given age. It is how the ones
/// with a short life are collected without reading them: a file that old
/// cannot be alive any more. It answers how many it dropped.
pub fn sweep(folder: &str, older: Duration) -> usize {
    let Some(base) = cache_dir() else {
        return 0;
    };

    let Ok(items) = fs::read_dir(base.join(folder)) else {
        return 0;
    };

    let cutoff = SystemTime::now().checked_sub(older).unwrap_or(UNIX_EPOCH);
    let mut dropped = 0;
    for item in items.flatten() {
        if item.path().extension() != Some(OsStr::new("json")) {
            continue;
        }

        let dead = item
            .metadata()
            .and_then(|metadata| metadata.modified())
            .is_ok_and(|at| at < cutoff);
        if dead && fs::remove_file(item.path()).is_ok() {
            dropped += 1;
        }
    }

    dropped
}

/// Removes the temporary files a crash left behind. A write that died between
/// the temporary file and the rename leaves it, and nothing else would ever
/// collect it. A temporary file this run wrote is still in flight, so it stays.
fn sweep_leftovers() {
    let Some(root) = cache_dir() else {
        return;
    };

    let started = *STARTED.get_or_init(SystemTime::now);
    drop_leftovers(&root, started);
}

fn drop_leftovers(dir: &Path, started: SystemTime) {
    let Ok(items) = fs::read_dir(dir) else {
        return;
    };

    for item in items.flatten() {
        let path = item.path();
        if path.is_dir() {
            drop_leftovers(&path, started);
            continue;
        }

        if path.extension() != Some(OsStr::new("tmp")) {
            continue;
        }

        let dead = item
            .metadata()
            .and_then(|metadata| metadata.modified())
            .is_ok_and(|at| at < started);
        if dead {
            let _ = fs::remove_file(path);
        }
    }
}

/// Keeps a folder under a ceiling, dropping the oldest files first.
pub fn trim(folder: &str, ext: Option<&str>, limit: usize) {
    let Some(base) = cache_dir() else {
        return;
    };

    let Ok(items) = fs::read_dir(base.join(folder)) else {
        return;
    };

    let mut entries: Vec<(PathBuf, SystemTime)> = items
        .flatten()
        .filter(|item| {
            ext.is_none_or(|extension| {
                item.path().extension().and_then(|value| value.to_str()) == Some(extension)
            })
        })
        .filter_map(|item| {
            item.metadata()
                .ok()
                .and_then(|metadata| metadata.modified().ok())
                .map(|modified| (item.path(), modified))
        })
        .collect();

    if entries.len() <= limit {
        return;
    }

    entries.sort_by_key(|entry| entry.1);
    let excess = entries.len() - limit;
    for (path, _) in entries.into_iter().take(excess) {
        let _ = fs::remove_file(path);
    }
}

#[cfg(test)]
#[path = "cache.test.rs"]
mod tests;
