use std::collections::HashSet;

use serde_json::Value;
use url::Url;

use crate::shared::music::domain::{Playlist, Song, Source, youtube_song};

use super::extract_videos::*;

pub(super) fn scrape(html: &str, field: &str) -> String {
    let needle = format!("\"{field}\":\"");
    let Some(start) = html.find(&needle) else {
        return String::new();
    };

    let rest = &html[start + needle.len()..];
    let Some(end) = rest.find('"') else {
        return String::new();
    };
    rest[..end].to_owned()
}

/// The page's own `ytInitialData`, the tree every browse answer is cut from.
pub(super) fn initial_data(html: &str) -> Option<Value> {
    let marker = html.find("ytInitialData")?;
    let start = html[marker..].find('{')? + marker;
    serde_json::Deserializer::from_str(&html[start..])
        .into_iter::<Value>()
        .next()?
        .ok()
}

pub(super) fn origin(url: &str) -> String {
    Url::parse(url)
        .ok()
        .and_then(|url| {
            let host = url.host_str()?;
            Some(format!("{}://{}", url.scheme(), host))
        })
        .unwrap_or_else(|| "https://www.youtube.com".to_owned())
}

pub(super) fn extract_playlists(value: &Value) -> Vec<Playlist> {
    let mut playlists = Vec::new();
    let sections = value
        .pointer("/contents/sectionListRenderer/contents")
        .or_else(|| {
            value.pointer(
                "/contents/twoColumnSearchResultsRenderer/primaryContents/sectionListRenderer/contents",
            )
        })
        .and_then(Value::as_array);
    let Some(sections) = sections else {
        return playlists;
    };

    for section in sections {
        let Some(items) = section
            .pointer("/itemSectionRenderer/contents")
            .and_then(Value::as_array)
        else {
            continue;
        };

        for item in items {
            let renderer = item
                .get("compactPlaylistRenderer")
                .or_else(|| item.get("playlistRenderer"));
            if let Some(playlist) = renderer.and_then(rendered) {
                playlists.push(playlist);
            }
        }
    }

    playlists
}

pub(super) fn rendered(renderer: &Value) -> Option<Playlist> {
    let id = renderer.get("playlistId").and_then(Value::as_str)?;
    let name = renderer
        .get("title")
        .and_then(renderer_text)
        .unwrap_or_default();
    let owner = renderer
        .get("shortBylineText")
        .or_else(|| renderer.get("ownerText"))
        .and_then(renderer_text)
        .unwrap_or_default();
    let track_count = renderer
        .get("videoCountShortText")
        .or_else(|| renderer.get("videoCountText"))
        .and_then(renderer_text)
        .and_then(|count| count.trim().parse().ok())
        .unwrap_or_default();
    Some(Playlist {
        id: format!("youtube:playlist:{id}"),
        name,
        subtitle: (!owner.is_empty()).then(|| owner.clone()),
        cover: thumbnails(renderer.get("thumbnail")),
        source: Source::YouTube,
        owner: (!owner.is_empty()).then_some(owner),
        track_count,
        ..Default::default()
    })
}

/// The account library arrives as `lockupViewModel` cards on every tab of the
/// YouTube page, so it is found by walking the answer instead of by path.
pub(super) fn extract_library(value: &Value) -> Vec<Playlist> {
    let mut playlists = Vec::new();
    library_items(value, &mut playlists);
    playlists
}

pub(super) fn library_items(value: &Value, out: &mut Vec<Playlist>) {
    match value {
        Value::Object(map) => {
            let entry = map.get("lockupViewModel").and_then(lockup).or_else(|| {
                map.get("gridPlaylistRenderer")
                    .or_else(|| map.get("compactPlaylistRenderer"))
                    .or_else(|| map.get("playlistRenderer"))
                    .and_then(rendered)
            });
            if let Some(playlist) = entry
                && !out.iter().any(|item| item.id == playlist.id)
            {
                out.push(playlist);
            }

            for value in map.values() {
                library_items(value, out);
            }
        }
        Value::Array(items) => {
            for value in items {
                library_items(value, out);
            }
        }
        _ => {}
    }
}

pub(super) fn lockup(view: &Value) -> Option<Playlist> {
    let kind = view.get("contentType").and_then(Value::as_str)?;
    if !kind.contains("PLAYLIST") && !kind.contains("COURSE") && !kind.contains("PODCAST") {
        return None;
    }

    let id = view.get("contentId").and_then(Value::as_str)?;
    let name = view
        .pointer("/metadata/lockupMetadataViewModel/title/content")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let owner = view
        .pointer(
            "/metadata/lockupMetadataViewModel/metadata/contentMetadataViewModel/metadataRows/0/metadataParts/0/text/content",
        )
        .and_then(Value::as_str)
        .unwrap_or_default();
    Some(Playlist {
        id: format!("youtube:playlist:{id}"),
        name: name.to_owned(),
        subtitle: (!owner.is_empty()).then(|| owner.to_owned()),
        cover: image(view),
        source: Source::YouTube,
        ..Default::default()
    })
}

pub(super) fn image(value: &Value) -> Option<String> {
    match value {
        Value::Object(map) => {
            if let Some(list) = map.get("sources").and_then(Value::as_array)
                && let Some(url) = list
                    .iter()
                    .rev()
                    .find_map(|item| item.get("url").and_then(Value::as_str))
            {
                return Some(url.to_owned());
            }

            map.values().find_map(image)
        }
        Value::Array(items) => items.iter().find_map(image),
        _ => None,
    }
}

pub(super) fn collect_playlist_videos(
    value: &Value,
    owner: &str,
    album: &str,
    out: &mut Vec<Song>,
    seen: &mut HashSet<String>,
) {
    match value {
        Value::Object(map) => {
            if let Some(renderer) = map.get("playlistVideoRenderer") {
                if let Some(song) = playlist_song(renderer, owner, album, seen) {
                    out.push(song);
                }
            } else if let Some(renderer) = map.get("videoRenderer")
                && let Some(song) = playlist_song(renderer, owner, album, seen)
            {
                out.push(song);
            } else if let Some(renderer) = map.get("lockupViewModel")
                && let Some(song) = lockup_song(renderer, owner, album, seen)
            {
                out.push(song);
            }

            for value in map.values() {
                collect_playlist_videos(value, owner, album, out, seen);
            }
        }
        Value::Array(items) => {
            for value in items {
                collect_playlist_videos(value, owner, album, out, seen);
            }
        }
        _ => {}
    }
}

pub(super) fn playlist_song(
    renderer: &Value,
    owner: &str,
    album: &str,
    seen: &mut HashSet<String>,
) -> Option<Song> {
    let id = renderer.get("videoId").and_then(Value::as_str)?;
    let title = renderer
        .get("title")
        .or_else(|| renderer.get("headline"))
        .and_then(renderer_text)?;
    if title.is_empty()
        || title == "[Private video]"
        || title == "[Deleted video]"
        || title == "Private video"
        || title == "Deleted video"
        || !seen.insert(id.to_owned())
    {
        return None;
    }

    let byline = renderer
        .get("shortBylineText")
        .or_else(|| renderer.get("ownerText"))
        .or_else(|| renderer.get("longBylineText"));
    let author = byline
        .and_then(renderer_text)
        .filter(|author| !author.is_empty())
        .unwrap_or_else(|| owner.to_owned());
    let duration = renderer
        .get("lengthSeconds")
        .and_then(Value::as_str)
        .and_then(|seconds| seconds.parse::<u32>().ok())
        .or_else(|| {
            renderer
                .get("lengthText")
                .and_then(renderer_text)
                .as_deref()
                .map(parse_duration)
        })
        .unwrap_or(0);
    let mut song = youtube_song(id, &title, &author, duration);
    song.album = album.to_owned();
    song.artist_ids = byline.and_then(browse_id).into_iter().collect();
    Some(song)
}

pub(super) fn lockup_song(
    view: &Value,
    owner: &str,
    album: &str,
    seen: &mut HashSet<String>,
) -> Option<Song> {
    if view.get("contentType").and_then(Value::as_str) != Some("LOCKUP_CONTENT_TYPE_VIDEO") {
        return None;
    }

    let id = view.get("contentId").and_then(Value::as_str)?;
    let metadata = view.pointer("/metadata/lockupMetadataViewModel")?;
    let title = metadata
        .pointer("/title/content")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if title.is_empty() || !seen.insert(id.to_owned()) {
        return None;
    }

    let author = metadata
        .pointer("/metadata/contentMetadataViewModel/metadataRows/0/metadataParts/0/text/content")
        .and_then(Value::as_str)
        .filter(|author| !author.is_empty())
        .unwrap_or(owner);
    let duration = badge(view)
        .as_deref()
        .map(parse_duration)
        .unwrap_or_default();
    let mut song = youtube_song(id, title, author, duration);
    song.album = album.to_owned();
    song.artist_ids = metadata
        .pointer("/metadata/contentMetadataViewModel/metadataRows/0/metadataParts/0")
        .and_then(nested_browse_id)
        .into_iter()
        .collect();
    Some(song)
}

pub(super) fn badge(value: &Value) -> Option<String> {
    match value {
        Value::Object(map) => {
            if let Some(text) = map
                .get("thumbnailBadgeViewModel")
                .and_then(|badge| badge.get("text"))
                .and_then(Value::as_str)
                && text.contains(':')
            {
                return Some(text.to_owned());
            }

            map.values().find_map(badge)
        }
        Value::Array(items) => items.iter().find_map(badge),
        _ => None,
    }
}

pub(super) fn find_continuation(value: &Value) -> Option<String> {
    match value {
        Value::Object(map) => {
            if let Some(token) = map
                .get("nextContinuationData")
                .and_then(|data| data.get("continuation"))
                .and_then(Value::as_str)
                .filter(|token| !token.is_empty())
            {
                return Some(token.to_owned());
            }

            if let Some(token) = map
                .get("continuationCommand")
                .and_then(|data| data.get("token"))
                .and_then(Value::as_str)
                .filter(|token| !token.is_empty())
            {
                return Some(token.to_owned());
            }

            map.values().find_map(find_continuation)
        }
        Value::Array(items) => items.iter().find_map(find_continuation),
        _ => None,
    }
}
