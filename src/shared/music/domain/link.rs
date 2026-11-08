use url::Url;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum YoutubeRef {
    Video(String),
    Playlist(String),
    Channel(String),
}

pub(crate) fn youtube_video(value: &str) -> bool {
    value.len() == 11
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

pub(crate) fn youtube_playlist(value: &str) -> bool {
    ["PL", "LL", "FL", "RD", "UU", "OL", "PU"]
        .iter()
        .any(|prefix| value.starts_with(prefix))
        && value.len() >= 12
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

pub(crate) fn youtube_channel(value: &str) -> bool {
    value.len() == 24
        && value.starts_with("UC")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

pub(crate) fn youtube_url(text: &str) -> Option<Url> {
    let url = if text.contains("://") {
        Url::parse(text).ok()?
    } else {
        Url::parse(&format!("https://{text}")).ok()?
    };

    let host = url
        .host_str()?
        .trim_start_matches("www.")
        .trim_start_matches("music.")
        .trim_start_matches("m.");
    matches!(host, "youtu.be" | "youtube.com" | "youtube-nocookie.com").then_some(url)
}

/// The playlist a link carries, even when it also names a video: a link copied
/// from inside a playlist means the playlist.
pub(crate) fn youtube_list(text: &str) -> Option<String> {
    youtube_url(text.trim())?
        .query_pairs()
        .find(|(key, _)| key == "list")
        .map(|(_, value)| value.into_owned())
        .filter(|value| !value.is_empty())
}

pub(crate) fn youtube_ref(text: &str) -> Option<YoutubeRef> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }

    if text.starts_with('@') {
        return Some(YoutubeRef::Channel(text.to_owned()));
    }

    if youtube_channel(text) {
        return Some(YoutubeRef::Channel(text.to_owned()));
    }

    if youtube_video(text) {
        return Some(YoutubeRef::Video(text.to_owned()));
    }

    if youtube_playlist(text) {
        return Some(YoutubeRef::Playlist(text.to_owned()));
    }

    let url = youtube_url(text)?;
    let host = url
        .host_str()?
        .trim_start_matches("www.")
        .trim_start_matches("music.")
        .trim_start_matches("m.");
    if let Some(value) = url
        .query_pairs()
        .find(|(key, _)| key == "v")
        .map(|(_, value)| value.into_owned())
        && youtube_video(&value)
    {
        return Some(YoutubeRef::Video(value));
    }

    if host == "youtu.be" {
        let id = url.path().trim_start_matches('/');
        if youtube_video(id) {
            return Some(YoutubeRef::Video(id.to_owned()));
        }
    }

    let parts: Vec<&str> = url
        .path()
        .split('/')
        .filter(|part| !part.is_empty())
        .collect();
    let kind = parts.first().copied().unwrap_or_default();
    let id = parts.get(1).copied().unwrap_or_default();
    if kind.starts_with('@') {
        return Some(YoutubeRef::Channel(kind.to_owned()));
    }

    if kind == "channel" && youtube_channel(id) {
        return Some(YoutubeRef::Channel(id.to_owned()));
    }

    if kind == "c" || kind == "user" {
        return Some(YoutubeRef::Channel(url.to_string()));
    }

    if matches!(kind, "shorts" | "embed" | "live") && youtube_video(id) {
        return Some(YoutubeRef::Video(id.to_owned()));
    }

    url.query_pairs()
        .find(|(key, _)| key == "list")
        .map(|(_, value)| value.into_owned())
        .filter(|value| youtube_playlist(value))
        .map(YoutubeRef::Playlist)
}

pub(crate) fn direct_url(text: &str) -> bool {
    Url::parse(text).is_ok_and(|url| matches!(url.scheme(), "http" | "https"))
}

pub(crate) fn spotify_link(text: &str) -> Option<(&'static str, String)> {
    let text = text.trim();
    if let Some(rest) = text.strip_prefix("spotify:") {
        let (kind, id) = rest.split_once(':')?;
        return spotify_ref(kind, id);
    }

    let url = if text.contains("://") {
        Url::parse(text).ok()?
    } else {
        Url::parse(&format!("https://{text}")).ok()?
    };

    let host = url.host_str()?.trim_start_matches("www.");
    if !matches!(host, "open.spotify.com" | "play.spotify.com") {
        return None;
    }

    let mut parts = url.path().split('/').filter(|part| !part.is_empty());
    if parts
        .clone()
        .next()
        .is_some_and(|part| part.starts_with("intl-"))
    {
        parts.next();
    }

    if parts.clone().next() == Some("embed") {
        parts.next();
    }

    spotify_ref(parts.next()?, parts.next()?)
}

pub(crate) fn spotify_ref(kind: &str, id: &str) -> Option<(&'static str, String)> {
    let kind = match kind {
        "track" => "track",
        "album" => "album",
        "artist" => "artist",
        "playlist" => "playlist",
        "user" => "user",
        _ => return None,
    };

    let id = id.split(['?', '#']).next().unwrap_or(id).trim().to_owned();
    if id.is_empty() {
        return None;
    }

    if kind != "user" && !spotify_id(&id) {
        return None;
    }

    Some((kind, id))
}

pub(crate) fn spotify_id(text: &str) -> bool {
    matches!(text.len(), 21..=22) && text.bytes().all(|byte| byte.is_ascii_alphanumeric())
}
