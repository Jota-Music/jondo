use std::collections::HashSet;

use serde_json::{Value, json};

use crate::shared::music::domain::{ChannelView, Playlist};
use crate::shared::storage::cache::{self, wire};

use super::*;
use super::{client::*, extract_playlists::*, extract_videos::*};

/// The channel a set of files holds, when all three are there, and when it was
/// saved. A channel with no playlists still writes an empty list, so a missing
/// file is a miss.
pub fn cached_channel(channel: &str) -> Option<(ChannelView, i64)> {
    let id = resolved(channel)?;
    let (info, saved) = cache::get_with_meta::<wire::Channel>(&format!("youtube/channels/{id}"))?;
    let videos = cache::get::<Vec<wire::Song>>(&format!("youtube/videos/{id}"))?;
    let playlists = cache::get::<Vec<wire::Summary>>(&format!("youtube/playlists/{id}"))?;

    Some((
        ChannelView {
            id: info.id,
            name: info.name,
            avatar: (!info.avatar.is_empty()).then_some(info.avatar),
            videos: videos.into_iter().map(Into::into).collect(),
            playlists: playlists
                .into_iter()
                .map(wire::Summary::into_domain)
                .collect(),
        },
        saved,
    ))
}

/// The channel id a name stands for, without a browse request: a browse id is
/// one already, and a handle was kept the first time it was resolved.
pub(super) fn resolved(channel: &str) -> Option<String> {
    let channel = channel
        .trim()
        .strip_prefix("youtube:channel:")
        .unwrap_or(channel.trim())
        .trim();
    if channel.is_empty() {
        return None;
    }

    if channel.len() == 24 && channel.starts_with("UC") {
        return Some(channel.to_owned());
    }

    cache::get::<String>(&format!("youtube/ids/{channel}"))
}

/// Keeps the channel, one file per thing the page shows.
fn remember(id: &str, view: &ChannelView) {
    let info = wire::Channel {
        id: view.id.clone(),
        name: view.name.clone(),
        avatar: view.avatar.clone().unwrap_or_default(),
    };
    let videos: Vec<wire::Song> = view.videos.iter().map(wire::Song::from).collect();
    let playlists: Vec<wire::Summary> = view.playlists.iter().map(wire::Summary::from).collect();

    for (name, result) in [
        (
            format!("youtube/channels/{id}"),
            cache::put(&format!("youtube/channels/{id}"), &info, None),
        ),
        (
            format!("youtube/videos/{id}"),
            cache::put(&format!("youtube/videos/{id}"), &videos, None),
        ),
        (
            format!("youtube/playlists/{id}"),
            cache::put(&format!("youtube/playlists/{id}"), &playlists, None),
        ),
    ] {
        if let Err(error) = result {
            log::debug!("youtube {name}: {error}");
        }
    }
}

impl Youtube {
    pub(super) async fn resolve_channel(&self, channel: &str) -> Result<String, String> {
        let channel = crate::shared::music::domain::id::bare(
            channel
                .trim()
                .strip_prefix("youtube:channel:")
                .unwrap_or(channel.trim()),
        )
        .trim();
        if channel.is_empty() {
            return Err("channel is required".to_owned());
        }

        if channel.len() == 24 && channel.starts_with("UC") {
            return Ok(channel.to_owned());
        }

        if let Some(id) = resolved(channel) {
            return Ok(id);
        }

        let key = format!("youtube/ids/{channel}");

        let target = if channel.contains("://") {
            channel.to_owned()
        } else {
            format!("https://www.youtube.com/{channel}")
        };

        let mut last_error = None;
        for client in CLIENTS {
            let payload = json!({
                "url": &target,
                "context": {"client": self.context(client)},
            });
            let value = match self.post(client, RESOLVE_URL, payload, false).await {
                Ok(value) => value,
                Err(error) => {
                    last_error = Some(error);
                    continue;
                }
            };

            if let Some(id) = value
                .pointer("/endpoint/browseEndpoint/browseId")
                .and_then(Value::as_str)
                .filter(|id| !id.is_empty())
            {
                let _ = cache::put(&key, &id.to_owned(), None);
                return Ok(id.to_owned());
            }

            last_error = Some(format!("channel not found: {channel}"));
        }

        Err(last_error.unwrap_or_else(|| "channel not found".to_owned()))
    }

    pub async fn channel(&self, id: &str) -> Result<ChannelView, String> {
        let id = self.resolve_channel(id).await?;
        if id.is_empty() {
            return Err("channel is required".to_owned());
        }

        let mut last_error = None;
        for client in CLIENTS {
            let payload = json!({
                "context": {"client": self.context(client)},
                "browseId": &id,
                "params": CHANNEL_VIDEOS,
            });
            let value = match self.post(client, BROWSE_URL, payload, false).await {
                Ok(value) => value,
                Err(error) => {
                    last_error = Some(error);
                    continue;
                }
            };

            let mut name = value
                .pointer("/metadata/channelMetadataRenderer/title")
                .and_then(Value::as_str)
                .filter(|name| !name.is_empty())
                .map(str::to_owned)
                .or_else(|| {
                    value
                        .pointer("/header/c4TabbedHeaderRenderer/title")
                        .and_then(Value::as_str)
                        .filter(|name| !name.is_empty())
                        .map(str::to_owned)
                })
                .unwrap_or_default();
            let mut avatar = thumbnails(value.pointer("/metadata/channelMetadataRenderer/avatar"))
                .or_else(|| thumbnails(value.pointer("/header/c4TabbedHeaderRenderer/avatar")));
            if (name.is_empty() || avatar.is_none())
                && let Ok(info) = self.channel_oembed(&id).await
            {
                if name.is_empty() {
                    name = info.0;
                }

                if avatar.is_none() {
                    avatar = info.1;
                }
            }

            if name.is_empty() {
                name = id.to_owned();
            }

            let mut videos = Vec::new();
            let mut seen = HashSet::new();
            let mut continuation: Option<String> = None;
            let mut previous = None;
            for page in 0..40 {
                let answer = if page == 0 {
                    value.clone()
                } else {
                    let Some(token) = continuation.clone() else {
                        break;
                    };

                    let payload = json!({
                        "context": {"client": self.context(client)},
                        "continuation": token,
                    });
                    match self.post(client, BROWSE_URL, payload, false).await {
                        Ok(answer) => answer,
                        Err(_) => break,
                    }
                };

                collect_videos(&answer, &name, &id, &mut videos, &mut seen);
                continuation = find_continuation(&answer);
                if continuation.is_none() || continuation == previous {
                    break;
                }

                previous = continuation.clone();
            }

            let playlists = self
                .channel_playlists(client, &id)
                .await
                .unwrap_or_default();

            let view = ChannelView {
                id: crate::shared::music::domain::id::channel(&id),
                name,
                avatar,
                videos: videos.iter().map(Video::song).collect(),
                playlists,
            };
            remember(&id, &view);
            return Ok(view);
        }

        Err(last_error.unwrap_or_else(|| "cannot open this channel".to_owned()))
    }

    pub(super) async fn channel_playlists(
        &self,
        client: Client,
        id: &str,
    ) -> Result<Vec<Playlist>, String> {
        let payload = json!({
            "context": {"client": self.context(client)},
            "browseId": id,
            "params": CHANNEL_PLAYLISTS,
        });
        let value = self.post(client, BROWSE_URL, payload, false).await?;
        Ok(extract_library(&value))
    }

    pub(super) async fn channel_oembed(
        &self,
        id: &str,
    ) -> Result<(String, Option<String>), String> {
        let url = format!(
            "https://www.youtube.com/oembed?format=json&url=https://www.youtube.com/channel/{id}"
        );
        let value: Value = self
            .http
            .get(url)
            .header(reqwest::header::USER_AGENT, BROWSER_AGENT)
            .send()
            .await
            .map_err(|error| error.to_string())?
            .json()
            .await
            .map_err(|error| error.to_string())?;
        Ok((
            text(value.get("title")).unwrap_or_else(|| id.to_owned()),
            text(value.get("thumbnail_url")),
        ))
    }
}
