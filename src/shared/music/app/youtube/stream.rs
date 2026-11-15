use std::time::{Duration, Instant};

use serde_json::{Value, json};

use super::*;
use super::{client::*, cookies::*, extract_playlists::*, extract_videos::*};
use crate::shared::kernel::app::mutex;
use crate::shared::music::domain::Song;
use crate::shared::storage::cache::{self, wire};

/// The video an earlier resolve named, rebuilt from the song it stored: the
/// title, the author, the channel and the duration are all on the file.
pub(super) fn video_from_song(video_id: &str, song: Song) -> Video {
    Video {
        id: crate::shared::music::domain::id::video(video_id),
        title: song.name,
        author: song.artists.first().cloned().unwrap_or_default(),
        channel: song.artist_ids.first().cloned(),
        duration: song.duration,
        official: false,
    }
}

/// The whole resolve is bounded: three clients, two rounds and a probe each can
/// outlive the user's patience when the network is only half there.
const RESOLVE: Duration = Duration::from_secs(25);

/// How recently a stream must have been resolved for its URL to be trusted
/// without a HEAD: the resolve already probed it, and the signature has hours
/// left. Older entries are still probed.
const PROBE_GRACE: i64 = 10 * 60 * 1000;

/// Whether a stream saved at that moment was probed recently enough to skip the
/// check. A HEAD to googlevideo costs a round trip, and paying it for a URL
/// this run just proved playable only slows the next track down.
fn proven(saved: i64) -> bool {
    crate::shared::kernel::app::clock::now_ms().saturating_sub(saved) < PROBE_GRACE
}

impl Youtube {
    pub async fn video(&self, video: &str) -> Result<Video, String> {
        let video_id = crate::shared::music::domain::id::bare(video);

        // a video resolved before answers from its own file, as Jota keeps it
        if let Some(song) = cache::get::<wire::Song>(&format!("youtube/tracks/{video_id}")) {
            return Ok(video_from_song(video_id, song.into()));
        }

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

        let video = Video {
            id: crate::shared::music::domain::id::video(video_id),
            title: text(value.get("title")).unwrap_or_else(|| video_id.to_owned()),
            author: text(value.get("author_name")).unwrap_or_default(),
            channel: value
                .get("author_url")
                .and_then(Value::as_str)
                .and_then(channel_ref)
                .map(|channel| crate::shared::music::domain::id::channel(&channel)),
            duration: 0,
            official: false,
        };

        let _ = cache::put(
            &format!("youtube/tracks/{video_id}"),
            &wire::Song::from(&video.song()),
            None,
        );

        Ok(video)
    }

    /// A URL from YouTube lasts hours, so the queue asks once. Replaying a
    /// song, skipping back or prefetching the next one costs no round trip.
    pub async fn stream(&self, video: &str) -> Result<Stream, String> {
        let video_id = crate::shared::music::domain::id::bare(video);
        if let Some((stream, saved)) = self.cache.stream(video_id) {
            // A URL resolved minutes ago was probed then and its signature has
            // hours left; an older one is probed before it is handed out. A URL
            // googlevideo now refuses past the first megabyte is not a stream:
            // re-resolve instead of handing out a link the player cannot finish.
            if proven(saved) || self.playable(&stream.url, &stream.agent).await {
                return Ok(stream);
            }
            self.forget(video_id);
        }

        self.refresh(video_id).await
    }

    /// A new URL for a stream that died mid-track: the cached one is the one
    /// that just failed, so retrying hands out the same broken link.
    pub async fn refresh(&self, video_id: &str) -> Result<Stream, String> {
        let video_id = crate::shared::music::domain::id::bare(video_id);
        let stream = tokio::time::timeout(RESOLVE, self.fetch(video_id))
            .await
            .map_err(|_| "YouTube took too long to answer".to_owned())??;
        self.cache.keep(video_id, &stream);
        Ok(stream)
    }

    pub fn cached(&self, video_id: &str) -> Option<Stream> {
        self.cache.stream(video_id).map(|(stream, _)| stream)
    }

    /// Drops the URL that just failed. A signed URL can be revoked hours before
    /// the expiry it advertises, and the cache would keep serving it to every
    /// later play of the song.
    pub fn forget(&self, video_id: &str) {
        self.cache.forget(video_id);
    }

    /// Drops the visitor token so the next resolve scrapes a fresh one: a stale
    /// token is what bot-checks every client.
    pub(super) fn forget_visitor(&self) {
        *mutex::lock(&self.visitor) = None;
        self.cache.forget_visitor();
    }

    /// The visitor scrape is one round trip to the homepage. Paying it at
    /// launch keeps it off the first track of the session.
    pub async fn warm(&self) {
        self.ensure_visitor().await;
    }

    async fn fetch(&self, video_id: &str) -> Result<Stream, String> {
        let mut last_error = None;
        let mut restricted = None;
        let mut refreshed = false;
        loop {
            let mut login = false;
            for round in 0..2 {
                let signed = round == 1;
                if signed && (!login || !self.signed_in()) {
                    break;
                }

                let clients: Vec<Client> = if signed { vec![WEB] } else { self.order() };
                for client in clients {
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
                                "YouTube did not accept the stored session for this video."
                                    .to_owned(),
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
                            restricted = Some(
                                "YouTube returned no playable audio for this video.".to_owned(),
                            );
                        } else if restricted.is_none() {
                            restricted = Some("YouTube returned no playable audio".to_owned());
                        }

                        continue;
                    };

                    if !self.playable(&url, client.agent).await {
                        last_error = Some("stream is not playable".to_owned());
                        continue;
                    }

                    self.cache.remember_client(client.name);

                    return Ok(Stream {
                        url,
                        agent: client.agent.to_owned(),
                        duration,
                    });
                }
            }

            // jota: a stale visitor bot-checks every client; refresh it once.
            if login && !refreshed {
                self.forget_visitor();
                self.ensure_visitor().await;
                refreshed = true;
                last_error = None;
                restricted = None;
                continue;
            }

            return Err(restricted
                .or(last_error)
                .unwrap_or_else(|| "cannot resolve audio".to_owned()));
        }
    }

    /// Whether the player can finish the track from this URL. A HEAD is not
    /// enough: a URL can answer it and still serve only its opening chunk, so
    /// the player reads the first bytes and then gets silence, or dies at the
    /// first re-resolved slice. The opening range gives both the proof and the
    /// length, and the tail is asked for with the same bytes the player will
    /// read when it looks for the movie box.
    pub(super) async fn playable(&self, url: &str, agent: &str) -> bool {
        let Some(advertised) = self.range(url, agent, 0, 1023).await else {
            return false;
        };
        let length = length_from_url(url)
            .filter(|length| *length > 0)
            .unwrap_or(advertised);

        length > 0
            && self
                .range(url, agent, length.saturating_sub(1024), length - 1)
                .await
                .is_some()
    }

    /// One bounded range, answered as the player will ask for it, carrying the
    /// length of the whole stream that the answer advertises.
    async fn range(&self, url: &str, agent: &str, start: u64, end: u64) -> Option<u64> {
        let response = self
            .http
            .get(url)
            .header(reqwest::header::RANGE, format!("bytes={start}-{end}"))
            .header(reqwest::header::USER_AGENT, agent)
            .header(reqwest::header::ACCEPT_ENCODING, "identity")
            .timeout(Duration::from_secs(6))
            .send()
            .await
            .ok()?;

        if response.status() != reqwest::StatusCode::PARTIAL_CONTENT {
            return None;
        }

        response
            .headers()
            .get(reqwest::header::CONTENT_RANGE)?
            .to_str()
            .ok()?
            .rsplit('/')
            .next()?
            .trim()
            .parse()
            .ok()
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

    /// The visitor token and the API key, refreshed at most once per window and
    /// never by two resolves at once: the queue prefetches in parallel, and a
    /// homepage download per resolve is bandwidth plus a bot-check signal.
    pub(super) async fn ensure_visitor(&self) {
        if self.visitor_ready() {
            return;
        }

        // The refresh lock is never taken on the request path: a slow homepage
        // holds this, not the resolves that already have a token.
        let _refreshing = self.refreshing.lock().await;

        // A resolve that waited for the lock may have refreshed already.
        if self.visitor_ready() {
            return;
        }

        // What a previous run kept is as good as a fresh scrape, but only when
        // nothing is in memory: adopting the same token again would keep it
        // alive past the window that makes it credible.
        if !self.has_visitor()
            && let Some(visitor) = self.cache.visitor()
        {
            *mutex::lock(&self.visitor) = Some(visitor);
            return;
        }

        match self.homepage().await {
            Some(visitor) => {
                self.cache.remember_visitor(&visitor);
                *mutex::lock(&self.visitor) = Some(visitor);
            }
            None => {
                // Degrade to whatever is cached and back off, so a bot-checked
                // homepage is not hammered on every resolve.
                let mut visitor = mutex::lock(&self.visitor);
                let blocked = Some(Instant::now() + VISITOR_BACKOFF);
                match visitor.as_mut() {
                    Some(visitor) => visitor.blocked = blocked,
                    None => {
                        *visitor = Some(Visitor {
                            blocked,
                            ..Default::default()
                        })
                    }
                }
            }
        }
    }

    fn visitor_ready(&self) -> bool {
        let now = Instant::now();
        mutex::lock(&self.visitor)
            .as_ref()
            .is_some_and(|visitor| visitor.ready(now))
    }

    fn has_visitor(&self) -> bool {
        mutex::lock(&self.visitor).is_some()
    }

    async fn homepage(&self) -> Option<Visitor> {
        let response = self
            .http
            .get(HOMEPAGE)
            .header(reqwest::header::USER_AGENT, CLIENTS[0].agent)
            .header(reqwest::header::ACCEPT_LANGUAGE, "en-US,en;q=0.9")
            .send()
            .await
            .ok()?;

        let html = response.text().await.ok()?;

        Some(Visitor {
            id: scrape(&html, "VISITOR_DATA"),
            key: scrape(&html, "INNERTUBE_API_KEY"),
            // A page without a token still counts as read: the next resolve
            // must not download it again to find the same nothing.
            at: Some(Instant::now()),
            blocked: None,
        })
    }
}
