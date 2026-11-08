use std::collections::HashSet;

use serde_json::Value;
use url::Url;

use crate::shared::music::domain::Channel;

use super::client::*;

pub(super) fn extract_channels(value: &Value) -> Vec<Channel> {
    let mut channels = Vec::new();
    let sections = value
        .pointer("/contents/sectionListRenderer/contents")
        .or_else(|| {
            value.pointer(
                "/contents/twoColumnSearchResultsRenderer/primaryContents/sectionListRenderer/contents",
            )
        })
        .and_then(Value::as_array);

    let Some(sections) = sections else {
        return channels;
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
                .get("compactChannelRenderer")
                .or_else(|| item.get("channelRenderer"));
            let Some(renderer) = renderer else {
                continue;
            };

            let Some(id) = renderer.get("channelId").and_then(Value::as_str) else {
                continue;
            };

            let name = renderer
                .get("displayName")
                .or_else(|| renderer.get("title"))
                .and_then(renderer_text)
                .unwrap_or_default();
            channels.push(Channel {
                id: id.to_owned(),
                name,
                avatar: thumbnails(renderer.get("thumbnail")),
            });
        }
    }

    channels
}

pub(super) fn renderer_text(value: &Value) -> Option<String> {
    value
        .get("simpleText")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .or_else(|| {
            value
                .pointer("/runs/0/text")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .filter(|text| !text.is_empty())
}

/// The channel a text run links to, carried by the first run's browse endpoint.
pub(super) fn browse_id(value: &Value) -> Option<String> {
    value
        .pointer("/runs/0/navigationEndpoint/browseEndpoint/browseId")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
}

/// The channel an oembed answer points at, either `/channel/<id>` or `@handle`.
pub(super) fn channel_ref(url: &str) -> Option<String> {
    let url = Url::parse(url).ok()?;
    let parts: Vec<&str> = url.path().trim_matches('/').split('/').collect();
    match parts.as_slice() {
        ["channel", id] if !id.is_empty() => Some((*id).to_owned()),
        [handle] if handle.starts_with('@') => Some((*handle).to_owned()),
        _ => None,
    }
}

/// The first channel id buried in a lockup metadata part, whose links nest
/// under `text` instead of the renderer root.
pub(super) fn nested_browse_id(value: &Value) -> Option<String> {
    match value {
        Value::Object(map) => map
            .get("browseId")
            .and_then(Value::as_str)
            .filter(|id| id.len() == 24 && id.starts_with("UC"))
            .map(str::to_owned)
            .or_else(|| map.values().find_map(nested_browse_id)),
        Value::Array(items) => items.iter().find_map(nested_browse_id),
        _ => None,
    }
}

pub(super) fn thumbnails(value: Option<&Value>) -> Option<String> {
    value
        .and_then(|value| value.get("thumbnails"))
        .and_then(Value::as_array)
        .and_then(|list| {
            list.iter()
                .rev()
                .find_map(|item| item.get("url").and_then(Value::as_str))
        })
        .map(|url| match url.strip_prefix("//") {
            Some(rest) => format!("https://{rest}"),
            None => url.to_owned(),
        })
}

pub(super) fn collect_videos(
    value: &Value,
    author: &str,
    channel: &str,
    out: &mut Vec<Video>,
    seen: &mut HashSet<String>,
) {
    match value {
        Value::Object(map) => {
            if let Some(id) = map.get("videoId").and_then(Value::as_str) {
                let title = map
                    .get("title")
                    .or_else(|| map.get("headline"))
                    .and_then(renderer_text)
                    .unwrap_or_default();
                if !title.is_empty() && seen.insert(id.to_owned()) {
                    let duration = map
                        .get("lengthText")
                        .and_then(renderer_text)
                        .as_deref()
                        .map(parse_duration)
                        .unwrap_or(0);
                    out.push(Video {
                        id: id.to_owned(),
                        title,
                        author: author.to_owned(),
                        channel: Some(channel.to_owned()),
                        duration,
                    });
                }
            }

            for value in map.values() {
                collect_videos(value, author, channel, out, seen);
            }
        }
        Value::Array(items) => {
            for value in items {
                collect_videos(value, author, channel, out, seen);
            }
        }
        _ => {}
    }
}

pub(super) fn extract_videos(value: &Value) -> Vec<Video> {
    let mut videos = Vec::new();
    let Some(sections) = value
        .pointer("/contents/sectionListRenderer/contents")
        .and_then(Value::as_array)
    else {
        return videos;
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
                .get("compactVideoRenderer")
                .or_else(|| item.get("videoRenderer"));
            let Some(renderer) = renderer else {
                continue;
            };

            let Some(id) = renderer.get("videoId").and_then(Value::as_str) else {
                continue;
            };

            let title = renderer
                .pointer("/title/runs/0/text")
                .or_else(|| renderer.pointer("/title/simpleText"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned();
            let byline = renderer
                .get("longBylineText")
                .or_else(|| renderer.get("shortBylineText"))
                .or_else(|| renderer.get("ownerText"));
            let author = byline
                .and_then(|text| text.pointer("/runs/0/text"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned();
            let channel = byline
                .and_then(|text| text.pointer("/runs/0/navigationEndpoint/browseEndpoint/browseId"))
                .and_then(Value::as_str)
                .filter(|id| !id.is_empty())
                .map(str::to_owned);
            let duration = renderer
                .pointer("/lengthText/simpleText")
                .or_else(|| renderer.pointer("/lengthText/runs/0/text"))
                .and_then(Value::as_str)
                .map(parse_duration)
                .unwrap_or(0);

            videos.push(Video {
                id: id.to_owned(),
                title,
                author,
                channel,
                duration,
            });
        }
    }

    videos
}

pub(super) fn pick_audio(value: &Value) -> Option<(String, f64)> {
    let mut best: Option<(&Value, (bool, bool, i64))> = None;
    for key in ["formats", "adaptiveFormats"] {
        let Some(formats) = value
            .pointer(&format!("/streamingData/{key}"))
            .and_then(Value::as_array)
        else {
            continue;
        };

        for format in formats {
            let mime = format.get("mimeType").and_then(Value::as_str).unwrap_or("");
            if !mime.starts_with("audio/mp4") {
                continue;
            }

            if format.get("url").and_then(Value::as_str).is_none() {
                continue;
            }

            let itag = format.get("itag").and_then(Value::as_i64).unwrap_or(0);
            let track = format.get("audioTrack");
            let original = !track
                .and_then(|track| track.get("isAutoDubbed"))
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let default = track
                .and_then(|track| track.get("audioIsDefault"))
                .and_then(Value::as_bool)
                .unwrap_or(true);
            let rank = (original, default, itag);
            if best.map(|(_, value)| rank > value).unwrap_or(true) {
                best = Some((format, rank));
            }
        }
    }

    let (format, _) = best?;
    let url = format.get("url")?.as_str()?.to_owned();
    let duration = duration_from_url(&url)
        .or_else(|| {
            format
                .get("approxDurationMs")
                .and_then(|value| value.as_str())
                .and_then(|value| value.parse::<f64>().ok())
                .map(|value| value / 1000.0)
        })
        .unwrap_or(0.0);

    Some((url, duration))
}

pub(super) fn duration_from_url(url: &str) -> Option<f64> {
    let parsed = Url::parse(url).ok()?;
    let value = parsed
        .query_pairs()
        .find(|(key, _)| key == "dur")
        .map(|(_, value)| value.to_string())?;
    value.parse::<f64>().ok()
}

pub(super) fn parse_duration(text: &str) -> u32 {
    let mut total = 0u32;
    for part in text.split(':') {
        let value = part.trim().parse::<u32>().unwrap_or(0);
        total = total.saturating_mul(60).saturating_add(value);
    }

    total
}

pub(super) fn video_id_from_query(query: &str) -> Option<String> {
    if query.len() == 11
        && query
            .chars()
            .all(|value| value.is_ascii_alphanumeric() || value == '-' || value == '_')
    {
        return Some(query.to_owned());
    }

    let url = Url::parse(query).ok()?;
    let host = url.host_str()?.trim_start_matches("www.");
    if host == "youtu.be" {
        let id = url.path().trim_matches('/');
        if id.len() == 11 {
            return Some(id.to_owned());
        }
    }

    if host.ends_with("youtube.com") {
        if let Some((_, value)) = url.query_pairs().find(|(key, _)| key == "v")
            && value.len() == 11
        {
            return Some(value.into_owned());
        }

        let parts: Vec<&str> = url.path().trim_matches('/').split('/').collect();
        if parts.len() == 2
            && matches!(parts[0], "shorts" | "embed" | "live")
            && parts[1].len() == 11
        {
            return Some(parts[1].to_owned());
        }
    }

    None
}

pub(super) fn text(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(str::to_owned)
        .filter(|value| !value.is_empty())
}
