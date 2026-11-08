use std::collections::HashSet;

use serde_json::{Value, json};

use crate::shared::music::domain::{ChannelView, Playlist};

use super::*;
use super::{client::*, extract_playlists::*, extract_videos::*};

impl Youtube {
    pub(super) async fn resolve_channel(&self, channel: &str) -> Result<String, String> {
        let channel = channel.trim();
        if channel.is_empty() {
            return Err("channel is required".to_owned());
        }

        if channel.len() == 24 && channel.starts_with("UC") {
            return Ok(channel.to_owned());
        }

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

            return Ok(ChannelView {
                id: id.clone(),
                name,
                avatar,
                videos: videos.iter().map(Video::song).collect(),
                playlists,
            });
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
