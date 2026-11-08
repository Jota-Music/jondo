use crate::shared::music::domain::{Song, Source};
use crate::shell::app::Route;

pub struct Part {
    pub text: String,
    pub route: Option<Route>,
}

impl Part {
    fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            route: None,
        }
    }

    fn link(text: impl Into<String>, route: Route) -> Self {
        Self {
            text: text.into(),
            route: Some(route),
        }
    }
}

pub fn parts(song: &Song, album: bool) -> Vec<Part> {
    let mut parts = Vec::new();
    for (index, name) in song.artists.iter().enumerate() {
        if index > 0 {
            parts.push(Part::plain(", "));
        }

        parts.push(match artist(song, index) {
            Some(route) => Part::link(name.clone(), route),
            None => Part::plain(name.clone()),
        });
    }

    if album && let Some(part) = album_part(song) {
        if !parts.is_empty() {
            parts.push(Part::plain(" • "));
        }

        parts.push(part);
    }

    if parts.is_empty() {
        parts.push(Part::plain(song.subtitle()));
    }

    parts
}

pub fn album_part(song: &Song) -> Option<Part> {
    if song.album.is_empty() {
        return None;
    }

    let route = song
        .playlist_id
        .as_ref()
        .filter(|id| !id.is_empty())
        .map(|id| Route::Playlist(format!("youtube:playlist:{id}")))
        .or_else(|| {
            song.album_id
                .as_ref()
                .filter(|id| !id.is_empty())
                .map(|id| Route::Album(id.clone()))
        });
    Some(match route {
        Some(route) => Part::link(song.album.clone(), route),
        None => Part::plain(song.album.clone()),
    })
}

fn artist(song: &Song, index: usize) -> Option<Route> {
    let id = song.artist_ids.get(index).filter(|id| !id.is_empty())?;
    Some(match song.source {
        Source::YouTube => Route::Channel(id.clone()),
        _ => Route::Artist(id.clone()),
    })
}
