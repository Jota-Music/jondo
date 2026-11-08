use std::time::{Duration, Instant};

use serde_json::{Value, json};

use super::*;
use super::{client::*, cookies::*, extract_playlists::*, extract_videos::*};
use crate::shared::kernel::app::mutex;

/// How long a resolved URL is trusted. YouTube hands out hours, but a short
/// window keeps a stale URL from surfacing as a 403 mid-session.
pub(super) const CACHED: Duration = Duration::from_secs(30 * 60);

impl Youtube {
    pub async fn video(&self, video_id: &str) -> Result<Video, String> {
        let url = format!(
            "https://www.youtube.com/oembed?format=json&url=https://www.youtube.com/watch?v={video_id}"
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

        Ok(Video {
            id: video_id.to_owned(),
            title: text(value.get("title")).unwrap_or_else(|| video_id.to_owned()),
            author: text(value.get("author_name")).unwrap_or_default(),
            channel: value
                .get("author_url")
                .and_then(Value::as_str)
                .and_then(channel_ref),
            duration: 0,
        })
    }

    /// A URL from YouTube lasts hours, so the queue asks once. Replaying a
    /// song, skipping back or prefetching the next one costs no round trip.
    pub async fn stream(&self, video_id: &str) -> Result<Stream, String> {
        if let Some(stream) = self.cached(video_id) {
            return Ok(stream);
        }

        let stream = self.fetch(video_id).await?;
        self.keep(video_id, &stream);
        Ok(stream)
    }

    pub fn cached(&self, video_id: &str) -> Option<Stream> {
        let streams = mutex::lock(&self.streams);
        let (stream, at) = streams.get(video_id)?;
        (at.elapsed() < CACHED).then(|| stream.clone())
    }

    fn keep(&self, video_id: &str, stream: &Stream) {
        let now = Instant::now();
        let mut streams = mutex::lock(&self.streams);
        streams.retain(|_, (_, at)| now.duration_since(*at) < CACHED);
        streams.insert(video_id.to_owned(), (stream.clone(), now));
    }

    /// The visitor scrape is one round trip to the homepage. Paying it at
    /// launch keeps it off the first track of the session.
    pub async fn warm(&self) {
        self.ensure_visitor().await;
    }

    async fn fetch(&self, video_id: &str) -> Result<Stream, String> {
        let mut last_error = None;
        let mut restricted = None;
        let mut login = false;
        for round in 0..2 {
            let signed = round == 1;
            if signed && (!login || !self.signed_in()) {
                break;
            }

            let clients: &[Client] = if signed { &[WEB] } else { &CLIENTS };
            for client in clients.iter().copied() {
                let payload = json!({
                    "videoId": video_id,
                    "context": {"client": self.context(client)},
                    "contentCheckOk": true,
                    "racyCheckOk": true,
                });

                let value = match self.post(client, PLAYER_URL, payload, signed).await {
                    Ok(value) => value,
                    Err(error) => {
                        last_error = Some(error);
                        continue;
                    }
                };

                let status = value
                    .pointer("/playabilityStatus/status")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let reason = value
                    .pointer("/playabilityStatus/reason")
                    .and_then(Value::as_str)
                    .filter(|reason| !reason.is_empty());
                if status == "LOGIN_REQUIRED" {
                    login = true;
                    if signed {
                        restricted = Some(
                            "YouTube did not accept the stored session for this video.".to_owned(),
                        );
                    } else if restricted.is_none() {
                        restricted = Some(match reason {
                            Some(reason) => format!("YouTube says: {reason}"),
                            None => "YouTube asks for a signed-in session".to_owned(),
                        });
                    }

                    continue;
                }

                if status != "OK" {
                    if !signed && restricted.is_none() {
                        restricted = Some(match reason {
                            Some(reason) => format!("YouTube says: {reason}"),
                            None => "YouTube could not play this video".to_owned(),
                        });
                    }

                    continue;
                }

                let Some((url, duration)) = pick_audio(&value) else {
                    if signed {
                        restricted =
                            Some("YouTube returned no playable audio for this video.".to_owned());
                    } else if restricted.is_none() {
                        restricted = Some("YouTube returned no playable audio".to_owned());
                    }

                    continue;
                };

                if !self.probe(&url, client.agent).await {
                    last_error = Some("stream is not playable".to_owned());
                    continue;
                }

                return Ok(Stream {
                    url,
                    agent: client.agent.to_owned(),
                    duration,
                });
            }
        }

        Err(restricted
            .or(last_error)
            .unwrap_or_else(|| "cannot resolve audio".to_owned()))
    }

    pub(super) async fn probe(&self, url: &str, agent: &str) -> bool {
        let response = self
            .http
            .head(url)
            .header(reqwest::header::USER_AGENT, agent)
            .header(reqwest::header::RANGE, "bytes=0-1")
            .timeout(Duration::from_secs(6))
            .send()
            .await;

        match response {
            Ok(response) => !matches!(response.status().as_u16(), 401 | 403 | 404 | 410),
            Err(_) => false,
        }
    }

    pub(super) fn context(&self, client: Client) -> Value {
        let mut context = json!({
            "clientName": client.name,
            "clientVersion": client.version,
            "deviceMake": client.device_make,
            "deviceModel": client.device_model,
            "osName": client.os_name,
            "osVersion": client.os_version,
            "hl": "en",
        });

        if let Some(visitor) = mutex::lock(&self.visitor).as_ref()
            && !visitor.id.is_empty()
        {
            context["visitorData"] = Value::String(visitor.id.clone());
        }

        context
    }

    pub(super) async fn post(
        &self,
        client: Client,
        url: &str,
        payload: Value,
        signed: bool,
    ) -> Result<Value, String> {
        self.ensure_visitor().await;

        let key = mutex::lock(&self.visitor)
            .as_ref()
            .map(|visitor| visitor.key.clone())
            .filter(|key| !key.is_empty())
            .unwrap_or_else(|| DEFAULT_KEY.to_owned());

        let origin = origin(url);
        let mut request = self
            .http
            .post(format!("{url}?key={key}"))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header(reqwest::header::ACCEPT, "*/*")
            .header(reqwest::header::ACCEPT_LANGUAGE, "en-US,en;q=0.9")
            .header(reqwest::header::USER_AGENT, client.agent)
            .header("X-YouTube-Client-Name", client.number.to_string())
            .header("X-YouTube-Client-Version", client.version)
            .header(reqwest::header::ORIGIN, &origin)
            .header(reqwest::header::REFERER, format!("{origin}/"))
            .json(&payload);

        if let Some(visitor) = mutex::lock(&self.visitor).as_ref()
            && !visitor.id.is_empty()
        {
            request = request.header("X-Goog-Visitor-Id", &visitor.id);
        }

        if signed
            && let Some(session) = mutex::lock(&self.session).as_ref()
            && let Some(hash) = sapisid_hash(&session.cookies, &origin)
        {
            request = request
                .header(reqwest::header::COOKIE, &session.cookies)
                .header("X-Goog-AuthUser", "0")
                .header("X-Origin", &origin)
                .header("Authorization", hash);
        }

        let mut last_error = None;
        for attempt in 0..3 {
            let response = match request
                .try_clone()
                .ok_or_else(|| "cannot retry request".to_owned())?
                .send()
                .await
            {
                Ok(response) => response,
                Err(error) => {
                    last_error = Some(error.to_string());
                    tokio::time::sleep(Duration::from_millis(150 + attempt as u64 * 250)).await;
                    continue;
                }
            };

            let status = response.status();
            let body = match response.json::<Value>().await {
                Ok(body) => body,
                Err(_) if !status.is_success() => Value::Null,
                Err(error) => return Err(error.to_string()),
            };

            if !status.is_success() {
                let detail = body
                    .pointer("/error/message")
                    .and_then(Value::as_str)
                    .filter(|detail| !detail.is_empty());
                let message = match detail {
                    Some(detail) => format!("YouTube answered {status}: {detail}"),
                    None => format!("YouTube answered {status}"),
                };

                if status.as_u16() == 429 || status.is_server_error() {
                    last_error = Some(message);
                    tokio::time::sleep(Duration::from_millis(500 + attempt as u64 * 500)).await;
                    continue;
                }

                return Err(message);
            }

            return Ok(body);
        }

        Err(last_error.unwrap_or_else(|| "YouTube request failed".to_owned()))
    }

    pub(super) async fn ensure_visitor(&self) {
        if mutex::lock(&self.visitor).is_some() {
            return;
        }

        let response = self
            .http
            .get(HOMEPAGE)
            .header(reqwest::header::USER_AGENT, CLIENTS[0].agent)
            .header(reqwest::header::ACCEPT_LANGUAGE, "en-US,en;q=0.9")
            .send()
            .await;

        let Ok(response) = response else {
            return;
        };

        let Ok(html) = response.text().await else {
            return;
        };

        let visitor = Visitor {
            id: scrape(&html, "VISITOR_DATA"),
            key: scrape(&html, "INNERTUBE_API_KEY"),
        };

        *mutex::lock(&self.visitor) = Some(visitor);
    }
}
