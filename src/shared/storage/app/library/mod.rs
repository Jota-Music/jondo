use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::shared::kernel::app::clock::now;
use crate::shared::music::domain::Playlist;

const LIMIT: usize = 48;
const REFRESH: u64 = 15 * 60;

#[derive(Serialize, Deserialize)]
struct Entry {
    saved: u64,
    playlist: Playlist,
}

#[derive(Serialize, Deserialize)]
struct Library {
    user: String,
    playlists: Vec<Playlist>,
}

use std::time::SystemTime;

fn root() -> Option<PathBuf> {
    crate::shared::storage::dir()
}

fn dir() -> Option<PathBuf> {
    Some(root()?.join("playlists"))
}

fn path(id: &str) -> Option<PathBuf> {
    Some(dir()?.join(format!("{}.json", id.replace(':', "_"))))
}

fn library_path() -> Option<PathBuf> {
    Some(root()?.join("library.json"))
}

pub fn get(id: &str) -> Option<(Playlist, bool)> {
    let entry: Entry = serde_json::from_str(&fs::read_to_string(path(id)?).ok()?).ok()?;
    let fresh = now().saturating_sub(entry.saved) < REFRESH;
    Some((entry.playlist, fresh))
}

pub fn put(playlist: &Playlist) {
    if playlist.songs.is_empty() {
        return;
    }

    let Some(dir) = dir() else {
        return;
    };

    let entry = Entry {
        saved: now(),
        playlist: playlist.clone(),
    };

    let Ok(text) = serde_json::to_string(&entry) else {
        log::warn!("cannot encode playlist {}", playlist.id);
        return;
    };

    let Some(path) = path(&playlist.id) else {
        return;
    };

    if write(&path, text) {
        trim(&dir, LIMIT);
    }
}

pub fn library(user: &str) -> Vec<Playlist> {
    if user.is_empty() {
        return Vec::new();
    }

    let Some(path) = library_path() else {
        return Vec::new();
    };

    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };

    match serde_json::from_str::<Library>(&text) {
        Ok(library) if library.user == user => library.playlists,
        _ => Vec::new(),
    }
}

pub fn save_library(user: &str, playlists: &[Playlist]) {
    if user.is_empty() || playlists.is_empty() {
        return;
    }

    let library = Library {
        user: user.to_owned(),
        playlists: playlists.to_vec(),
    };

    let Ok(text) = serde_json::to_string(&library) else {
        return;
    };

    if let Some(path) = library_path() {
        write(&path, text);
    }
}

pub fn forget_library() {
    if let Some(path) = library_path() {
        let _ = fs::remove_file(path);
    }
}

fn write(path: &Path, text: String) -> bool {
    if let Some(parent) = path.parent()
        && let Err(error) = fs::create_dir_all(parent)
    {
        log::warn!("cannot create {}: {error}", parent.display());
        return false;
    }

    if let Err(error) = fs::write(path, text) {
        log::warn!("cannot save {}: {error}", path.display());
        return false;
    }

    true
}

pub fn trim(dir: &Path, limit: usize) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    let mut files: Vec<(SystemTime, PathBuf)> = entries
        .flatten()
        .filter_map(|entry| Some((entry.metadata().ok()?.modified().ok()?, entry.path())))
        .collect();
    if files.len() <= limit {
        return;
    }

    files.sort_by_key(|entry| std::cmp::Reverse(entry.0));
    for (_, path) in files.drain(limit..) {
        let _ = fs::remove_file(path);
    }
}

#[cfg(test)]
#[path = "library.test.rs"]
mod tests;
