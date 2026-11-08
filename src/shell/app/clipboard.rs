use std::collections::HashSet;

use crate::shared::music::domain::link::{YoutubeRef, spotify_link, youtube_link, youtube_list};

use crate::shell::domain::Route;
pub use crate::shell::domain::link::{Kind, Link, Target};

/// A copied link resolves to the route it opens or to the song it plays.
/// Anything else (plain text, local ids, other hosts) is ignored.
pub fn parse(text: &str) -> Option<Link> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }

    if let Some((kind, id)) = spotify_link(text) {
        let (kind, target) = match kind {
            "track" => (Kind::Song, Target::Song(format!("spotify:track:{id}"))),
            "album" => (Kind::Album, Target::Route(Route::Album(id))),
            "artist" => (Kind::Artist, Target::Route(Route::Artist(id))),
            "playlist" => (
                Kind::Playlist,
                Target::Route(Route::Playlist(format!("spotify:playlist:{id}"))),
            ),
            "user" => (Kind::User, Target::Route(Route::User(id))),
            _ => return None,
        };

        return Some(Link {
            kind,
            text: text.to_owned(),
            target,
        });
    }

    // A link copied from inside a playlist carries both, and the playlist is
    // what was shared.
    if let Some(id) = youtube_list(text) {
        return Some(Link {
            kind: Kind::Playlist,
            text: text.to_owned(),
            target: Target::Route(Route::Playlist(format!("youtube:playlist:{id}"))),
        });
    }

    let (kind, target) = match youtube_link(text)? {
        YoutubeRef::Video(id) => (Kind::Song, Target::Song(format!("youtube:{id}"))),
        YoutubeRef::Playlist(id) => (
            Kind::Playlist,
            Target::Route(Route::Playlist(format!("youtube:playlist:{id}"))),
        ),
        YoutubeRef::Channel(id) => (Kind::Channel, Target::Route(Route::Channel(id))),
    };

    Some(Link {
        kind,
        text: text.to_owned(),
        target,
    })
}

/// The clipboard is read when the window gains focus, never polled: a selection
/// owner that never answers must not freeze the frame.
#[derive(Default)]
pub struct Clipboard {
    pub pending: Option<Link>,
    pub busy: bool,
    seen: HashSet<String>,
}

impl Clipboard {
    pub fn detect(&mut self, text: &str) -> bool {
        let text = text.trim();
        if text.is_empty() || !self.seen.insert(text.to_owned()) {
            return false;
        }

        let Some(link) = parse(text) else {
            return false;
        };

        self.pending = Some(link);
        self.busy = false;
        true
    }

    pub fn remember(&mut self, text: &str) {
        self.seen.insert(text.trim().to_owned());
    }

    pub fn dismiss(&mut self) {
        self.pending = None;
        self.busy = false;
    }
}
