use crate::shared::kernel::app::clock::now_ms;
use crate::shared::music::domain::Playlist;
use crate::shared::storage::cache::{self, wire};

/// How long a cached playlist paints before the source is asked again.
const REFRESH: i64 = 15 * 60 * 1000;

/// Where a user context is cached. A context has no revision to compare, so
/// Jota keeps the URI the source named and reads it back as it is. Liked Songs
/// is the one that matters here.
fn context(id: &str) -> Option<String> {
    let user = id.strip_prefix("spotify:collection:")?;
    Some(format!("spotify/contexts/spotify:user:{user}:collection"))
}

/// Where a playlist is cached, or nothing when it is a radio: Jota recomputes
/// a radio on every visit and never stores it.
fn name(id: &str) -> Option<String> {
    if id.starts_with("spotify:radio:") || context(id).is_some() {
        return None;
    }

    Some(format!("spotify/playlists/{id}"))
}

/// Where the playlists an account owns are cached. The user is part of the
/// name: two accounts on one device are two libraries.
fn library_name(user: &str) -> String {
    format!("spotify/library/{user}")
}

pub fn get(id: &str) -> Option<(Playlist, bool)> {
    if let Some(context) = context(id) {
        let playlist = cache::get::<wire::Playlist>(&context)?;
        return Some((playlist.into_domain(id), true));
    }

    let (cached, saved) = cache::get_with_meta::<wire::Revisioned>(&name(id)?)?;
    let playlist = cached.playlist.into_domain(id);
    Some((playlist, now_ms().saturating_sub(saved) < REFRESH))
}

pub fn put(playlist: &Playlist) {
    if playlist.songs.is_empty() {
        return;
    }

    if let Some(context) = context(&playlist.id) {
        if let Err(error) = cache::put(&context, &wire::Playlist::from(playlist), None) {
            log::warn!("cannot cache context {}: {error}", playlist.id);
        }
        return;
    }

    let Some(name) = name(&playlist.id) else {
        return;
    };

    let entry = wire::Revisioned {
        revision: String::new(),
        playlist: wire::Playlist::from(playlist),
    };

    if let Err(error) = cache::put(&name, &entry, None) {
        log::warn!("cannot cache playlist {}: {error}", playlist.id);
        return;
    }

    cache::trim("spotify/playlists", Some("json"), cache::PLAYLIST_LIMIT);
}

pub fn library(user: &str) -> Vec<Playlist> {
    if user.is_empty() {
        return Vec::new();
    }

    let Some(summaries) = cache::get::<Vec<wire::Summary>>(&library_name(user)) else {
        return Vec::new();
    };

    summaries
        .into_iter()
        .map(wire::Summary::into_domain)
        .collect()
}

pub fn save_library(user: &str, playlists: &[Playlist]) {
    if user.is_empty() || playlists.is_empty() {
        return;
    }

    let summaries: Vec<wire::Summary> = playlists.iter().map(wire::Summary::from).collect();
    if let Err(error) = cache::put(&library_name(user), &summaries, None) {
        log::warn!("cannot cache the library: {error}");
    }
}

pub fn forget_library(user: &str) {
    if user.is_empty() {
        return;
    }

    if let Err(error) = cache::forget(&library_name(user)) {
        log::warn!("cannot forget the library: {error}");
    }
}

#[cfg(test)]
#[path = "library.test.rs"]
mod tests;
