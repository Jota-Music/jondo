//! Every id in the app is `{platform}:{type}:{id}`.
//!
//! One place builds them and one place reads them, so a song, a playlist, an
//! artist and a channel answer the same question: which service, which kind of
//! thing, and which one. The functions here also read the shapes the app wrote
//! before the rule, so an old file, an old backup or an id pasted from another
//! app lands on the same name.

/// The service an id belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    Spotify,
    YouTube,
    Local,
}

/// What an id names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Track,
    Playlist,
    Album,
    Artist,
    User,
    Channel,
    Collection,
    Radio,
    Room,
}

impl Platform {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Spotify => "spotify",
            Self::YouTube => "youtube",
            Self::Local => "local",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "spotify" => Some(Self::Spotify),
            "youtube" => Some(Self::YouTube),
            "local" => Some(Self::Local),
            _ => None,
        }
    }
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Track => "track",
            Self::Playlist => "playlist",
            Self::Album => "album",
            Self::Artist => "artist",
            Self::User => "user",
            Self::Channel => "channel",
            Self::Collection => "collection",
            Self::Radio => "radio",
            Self::Room => "room",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "track" => Some(Self::Track),
            "playlist" => Some(Self::Playlist),
            "album" => Some(Self::Album),
            "artist" => Some(Self::Artist),
            "user" => Some(Self::User),
            "channel" => Some(Self::Channel),
            "collection" => Some(Self::Collection),
            "radio" => Some(Self::Radio),
            "room" => Some(Self::Room),
            _ => None,
        }
    }
}

/// `{platform}:{type}:{id}`
pub fn make(platform: Platform, kind: Kind, id: &str) -> String {
    format!("{}:{}:{}", platform.as_str(), kind.as_str(), id)
}

/// The three parts of an id, when it carries all three.
pub fn parts(value: &str) -> Option<(Platform, Kind, &str)> {
    let (platform, rest) = value.split_once(':')?;
    let (kind, id) = rest.split_once(':')?;
    Some((Platform::parse(platform)?, Kind::parse(kind)?, id))
}

/// The platform an id belongs to. An id without one belongs to this app.
pub fn platform(value: &str) -> Platform {
    value
        .split_once(':')
        .and_then(|(name, _)| Platform::parse(name))
        .unwrap_or(Platform::Local)
}

/// The part the service understands, with the prefixes taken off.
pub fn bare(value: &str) -> &str {
    parts(value).map(|(_, _, id)| id).unwrap_or(value)
}

/// The user a collection names, in either shape.
fn collection_user(value: &str) -> Option<&str> {
    value
        .strip_prefix("spotify:user:")
        .and_then(|rest| rest.strip_suffix(":collection"))
}

/// A song. A YouTube song is a video, and a local one belongs to this app.
pub fn track(value: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        return String::new();
    }

    if let Some(id) = value.strip_prefix("youtube:track:") {
        return make(Platform::YouTube, Kind::Track, id);
    }
    if let Some(id) = value.strip_prefix("youtube:") {
        return make(Platform::YouTube, Kind::Track, id);
    }
    if let Some(id) = value.strip_prefix("spotify:track:") {
        return make(Platform::Spotify, Kind::Track, id);
    }
    if let Some(id) = value.strip_prefix("local:track:") {
        return make(Platform::Local, Kind::Track, id);
    }
    if let Some(id) = value.strip_prefix("local:") {
        return make(Platform::Local, Kind::Track, id);
    }

    // a bare id: its shape says which service it belongs to
    if super::link::youtube_video(value) {
        return make(Platform::YouTube, Kind::Track, value);
    }
    if super::link::spotify_id(value) {
        return make(Platform::Spotify, Kind::Track, value);
    }

    make(Platform::Local, Kind::Track, value)
}

/// A YouTube video, the thing a song plays. A video is always YouTube's, so
/// it never falls back to another platform the way a song id does.
pub fn video(value: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        return String::new();
    }

    if let Some(id) = value.strip_prefix("youtube:track:") {
        return make(Platform::YouTube, Kind::Track, id);
    }

    make(Platform::YouTube, Kind::Track, value)
}

/// A playlist. The liked songs of a user and a radio are playlists too, so
/// they keep the kind that names them.
pub fn playlist(value: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        return String::new();
    }

    if let Some(user) = collection_user(value) {
        return make(Platform::Spotify, Kind::Collection, user);
    }
    if let Some(track) = value.strip_prefix("spotify:radio:track:") {
        return make(Platform::Spotify, Kind::Radio, track);
    }
    if let Some((platform, kind, id)) = parts(value)
        && matches!(kind, Kind::Collection | Kind::Radio)
    {
        return make(platform, kind, id);
    }
    if let Some(id) = value.strip_prefix("youtube:playlist:") {
        return make(Platform::YouTube, Kind::Playlist, id);
    }
    if let Some(id) = value.strip_prefix("spotify:playlist:") {
        return make(Platform::Spotify, Kind::Playlist, id);
    }
    if let Some(id) = value.strip_prefix("local:playlist:") {
        return make(Platform::Local, Kind::Playlist, id);
    }
    if let Some(id) = value.strip_prefix("local:") {
        return make(Platform::Local, Kind::Playlist, id);
    }

    // a bare id: its shape says which service it belongs to
    if super::link::youtube_playlist(value) {
        return make(Platform::YouTube, Kind::Playlist, value);
    }
    if super::link::spotify_id(value) {
        return make(Platform::Spotify, Kind::Playlist, value);
    }

    make(Platform::Local, Kind::Playlist, value)
}

/// An album.
pub fn album(value: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        return String::new();
    }

    if let Some(id) = value.strip_prefix("spotify:album:") {
        return make(Platform::Spotify, Kind::Album, id);
    }

    make(Platform::Spotify, Kind::Album, value)
}

/// An artist or a channel, whichever the platform names.
pub fn artist(value: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        return String::new();
    }

    if let Some(id) = value.strip_prefix("spotify:artist:") {
        return make(Platform::Spotify, Kind::Artist, id);
    }
    if let Some(id) = value.strip_prefix("artist:") {
        return make(Platform::Spotify, Kind::Artist, id);
    }
    if let Some(id) = value.strip_prefix("youtube:channel:") {
        return make(Platform::YouTube, Kind::Channel, id);
    }
    if let Some(id) = value.strip_prefix("youtube:") {
        return make(Platform::YouTube, Kind::Channel, id);
    }

    make(Platform::Spotify, Kind::Artist, value)
}

/// A user.
pub fn user(value: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        return String::new();
    }

    if let Some(id) = value.strip_prefix("spotify:user:") {
        if let Some(user) = id.strip_suffix(":collection") {
            return make(Platform::Spotify, Kind::Collection, user);
        }
        return make(Platform::Spotify, Kind::User, id);
    }
    if let Some(id) = value.strip_prefix("user:") {
        return make(Platform::Spotify, Kind::User, id);
    }

    make(Platform::Spotify, Kind::User, value)
}

/// A channel.
pub fn channel(value: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        return String::new();
    }

    if let Some(id) = value.strip_prefix("youtube:channel:") {
        return make(Platform::YouTube, Kind::Channel, id);
    }
    if let Some(id) = value.strip_prefix("youtube:") {
        return make(Platform::YouTube, Kind::Channel, id);
    }

    make(Platform::YouTube, Kind::Channel, value)
}

/// A room, named by the relay it lives in and its code.
pub fn room(value: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        return String::new();
    }

    if let Some(id) = value.strip_prefix("local:room:") {
        return make(Platform::Local, Kind::Room, id);
    }

    make(Platform::Local, Kind::Room, value)
}

#[cfg(test)]
#[path = "id.test.rs"]
mod tests;
