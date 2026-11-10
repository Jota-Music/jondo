use std::sync::Arc;

use crate::shared::music::domain::{Playlist, Source};

use super::library::context_pages;
use super::session::*;
use super::tracks::*;

const STATION: &str = "spotify:station:track:";
const RADIO: &str = "spotify:radio:track:";
const TRACK: &str = "spotify:track:";
/// A radio never reports the end, so two pages bound the listening session.
const PAGES: usize = 2;

/// Resolves the song-station context around a track as a synthetic playlist.
/// Radio is dynamic, so it is never cached: every open asks for a fresh set.
pub async fn radio(
    client: &reqwest::Client,
    session: &Arc<Session>,
    id: &str,
) -> Result<Playlist, String> {
    let id = seed(id);
    if id.is_empty() {
        return Err("Spotify could not open this radio".to_owned());
    }

    // Modern clients drive song radio through station contexts; the legacy
    // radio form is the fallback when a compatible station is unavailable.
    let mut uris = Vec::new();
    for prefix in [STATION, RADIO] {
        if let Ok(found) = context_pages(client, session, &format!("{prefix}{id}"), PAGES).await
            && !found.is_empty()
        {
            uris = found;
            break;
        }
    }

    if uris.is_empty() {
        return Err("Spotify could not open this radio".to_owned());
    }

    let songs = tracks(client, session, &uris).await;
    let track_count = songs.len() as u32;
    let name = seed_name(client, session, &id)
        .await
        .map(|name| format!("Radio · {name}"))
        .unwrap_or_else(|| "Radio".to_owned());
    let cover = songs.iter().find_map(|song| song.cover.clone());
    let covers = songs
        .iter()
        .filter_map(|song| song.cover.clone())
        .take(4)
        .collect();

    Ok(Playlist {
        id: format!("{RADIO}{id}"),
        name,
        cover,
        covers,
        source: Source::Spotify,
        songs,
        track_count,
        ..Default::default()
    })
}

/// The track a radio is built around, whichever form the caller sent.
pub(super) fn seed(id: &str) -> String {
    let mut id = id.trim();
    loop {
        match [RADIO, STATION, TRACK]
            .iter()
            .find_map(|prefix| id.strip_prefix(*prefix))
        {
            Some(rest) => id = rest,
            None => return id.to_owned(),
        }
    }
}

async fn seed_name(client: &reqwest::Client, session: &Arc<Session>, id: &str) -> Option<String> {
    let uri = format!("{TRACK}{id}");
    tracks(client, session, &[uri])
        .await
        .into_iter()
        .next()
        .filter(|song| !song.broken && !song.name.is_empty())
        .map(|song| song.name)
}
