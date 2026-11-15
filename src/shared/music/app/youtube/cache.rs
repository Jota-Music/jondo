use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use url::Url;

use super::BROWSER_AGENT;
use super::client::{CLIENTS, Visitor};
use crate::shared::kernel::app::mutex;
use crate::shared::music::domain::Stream;
use crate::shared::storage::cache::{self, wire};

/// How long a URL that carries no `expire` of its own is trusted. YouTube
/// always signs one, so this only covers a malformed answer.
const FALLBACK: u64 = 30 * 60;

/// Expiry is a whole second, so a URL is not trusted to its last instant: the
/// request would race the signature.
const MARGIN: u64 = 30;

/// How long the visitor token survives a restart. A cold start that resumes it
/// skips the homepage scrape entirely.
const VISITOR: i64 = 24 * 60 * 60 * 1000;

/// The token file: the visitor and the client that last played, nothing else.
/// It is raw JSON, with no envelope, so Jota reads the same file.
const SESSION: &str = "youtube/session";

/// Where the stream resolved for a video is kept: one file per video, so a play
/// rewrites one entry and not the whole YouTube cache.
fn stream_name(video_id: &str) -> String {
    format!("youtube/streams/{video_id}")
}

/// What YouTube handed out that is still worth something after a restart.
#[derive(Clone, Default, Serialize, Deserialize)]
struct Saved {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    visitor: Option<Token>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    client: String,
}

/// A value with a deadline, in milliseconds since the epoch.
#[derive(Clone, Serialize, Deserialize)]
struct Token {
    value: String,
    at: i64,
}

pub(super) struct Cache {
    /// False for a caller that keeps nothing between runs: the session and the
    /// streams stay in memory, so a test leaves no side effects.
    persist: bool,
    saved: Mutex<Saved>,
    /// What this run resolved, so the track that is about to play again costs
    /// no disk read.
    streams: Mutex<BTreeMap<String, (Stream, i64)>>,
}

impl Cache {
    pub(super) fn load(persist: bool) -> Self {
        let saved = if persist {
            read_session().unwrap_or_default()
        } else {
            Saved::default()
        };

        Self {
            persist,
            saved: Mutex::new(saved),
            streams: Mutex::new(BTreeMap::new()),
        }
    }

    /// A stream resolved in this run or a previous one, with the moment it was
    /// saved. A URL past its own expiry is never handed out.
    pub(super) fn stream(&self, video_id: &str) -> Option<(Stream, i64)> {
        let mut streams = mutex::lock(&self.streams);
        if let Some((stream, saved)) = streams.get(video_id)
            && now() < expiry(&stream.url)
        {
            return Some((stream.clone(), *saved));
        }
        streams.remove(video_id);
        drop(streams);

        if !self.persist {
            return None;
        }

        let (audio, saved) = cache::get_with_meta::<wire::Audio>(&stream_name(video_id))?;
        let stream = stream(&audio);
        mutex::lock(&self.streams).insert(video_id.to_owned(), (stream.clone(), saved));
        Some((stream, saved))
    }

    /// Keeps a stream for as long as its own URL lives.
    pub(super) fn keep(&self, video_id: &str, stream: &Stream) {
        let saved = cache::now_ms();
        mutex::lock(&self.streams).insert(video_id.to_owned(), (stream.clone(), saved));

        if !self.persist {
            return;
        }

        let ttl = Duration::from_secs(expiry(&stream.url).saturating_sub(now()));
        let audio = audio(video_id, stream);
        if let Err(error) = cache::put(&stream_name(video_id), &audio, Some(ttl)) {
            log::debug!("youtube stream: {error}");
        }
    }

    /// A URL that just failed is not handed out again: the next play of the
    /// same song would fail exactly the same way, and a signed URL is revoked
    /// long before the expiry it advertises.
    pub(super) fn forget(&self, video_id: &str) {
        mutex::lock(&self.streams).remove(video_id);

        if self.persist {
            let _ = cache::forget(&stream_name(video_id));
        }
    }

    /// Drops the persisted visitor token so the next resolve scrapes a fresh
    /// one: a stale token is what bot-checks every client.
    pub(super) fn forget_visitor(&self) {
        let mut saved = mutex::lock(&self.saved);
        if saved.visitor.take().is_some() {
            self.write(&saved);
        }
    }

    pub(super) fn visitor(&self) -> Option<Visitor> {
        let saved = mutex::lock(&self.saved);
        let token = saved.visitor.as_ref()?;
        (cache::now_ms().saturating_sub(token.at) < VISITOR)
            .then(|| Visitor::resumed(&token.value, ""))
    }

    pub(super) fn remember_visitor(&self, visitor: &Visitor) {
        let mut saved = mutex::lock(&self.saved);
        saved.visitor = Some(Token {
            value: visitor.id.clone(),
            at: cache::now_ms(),
        });
        self.write(&saved);
    }

    /// The client that last produced a playable stream. A session resumes with
    /// it instead of falling through the whole list per track.
    pub(super) fn client(&self) -> Option<String> {
        let saved = mutex::lock(&self.saved);
        (!saved.client.is_empty()).then(|| saved.client.clone())
    }

    pub(super) fn remember_client(&self, name: &str) {
        let mut saved = mutex::lock(&self.saved);
        if saved.client == name {
            return;
        }

        saved.client = name.to_owned();
        self.write(&saved);
    }

    /// Best effort: a cache that cannot be written costs a resolve, never the
    /// track that is playing.
    fn write(&self, saved: &Saved) {
        if !self.persist {
            return;
        }

        let Ok(text) = serde_json::to_string(saved) else {
            return;
        };

        if let Err(error) = cache::write_raw(SESSION, text.as_bytes()) {
            log::debug!("youtube session: {error}");
        }
    }
}

/// The session as a previous run left it: raw JSON, no envelope.
fn read_session() -> Option<Saved> {
    serde_json::from_slice::<Saved>(&cache::read_raw(SESSION)?).ok()
}

/// The file holds Jota's audio: no user agent, the client that resolved it
/// instead, which names the agent again on the way back.
fn audio(video_id: &str, stream: &Stream) -> wire::Audio {
    wire::Audio {
        url: stream.url.clone(),
        duration: stream.duration.max(0.0) as u32,
        expire_at: expiry(&stream.url) as i64,
        video_id: video_id.to_owned(),
        client_name: CLIENTS
            .iter()
            .find(|client| client.agent == stream.agent)
            .map(|client| client.name.to_owned())
            .unwrap_or_default(),
    }
}

fn stream(audio: &wire::Audio) -> Stream {
    let agent = CLIENTS
        .iter()
        .find(|client| client.name == audio.client_name)
        .map(|client| client.agent)
        .unwrap_or(BROWSER_AGENT);

    Stream {
        url: audio.url.clone(),
        agent: agent.to_owned(),
        duration: audio.duration as f64,
    }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0)
}

/// The URL carries its own expiry, and it is hours away: a flat window would
/// throw away a link that still works and pay a resolve for nothing.
fn expiry(url: &str) -> u64 {
    let signed = Url::parse(url).ok().and_then(|url| {
        url.query_pairs()
            .find(|(key, _)| key == "expire")
            .and_then(|(_, value)| value.parse::<u64>().ok())
    });

    match signed {
        Some(expire) => expire.saturating_sub(MARGIN).max(now()),
        None => now() + FALLBACK,
    }
}

#[cfg(test)]
#[path = "cache.test.rs"]
mod tests;
