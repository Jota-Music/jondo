use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use url::Url;

use super::client::Visitor;
use crate::shared::kernel::app::mutex;
use crate::shared::music::domain::Stream;

/// How long a URL that carries no `expire` of its own is trusted. YouTube
/// always signs one, so this only covers a malformed answer.
const FALLBACK: u64 = 30 * 60;

/// Expiry is a whole second, so a URL is not trusted to its last instant: the
/// request would race the signature.
const MARGIN: u64 = 30;

/// How long the visitor token survives a restart. A cold start that resumes it
/// skips the homepage scrape entirely.
const VISITOR: u64 = 24 * 60 * 60;

/// What YouTube handed out that is still worth something after a restart. The
/// file is a snapshot, never a source of truth: an entry past its expiry is
/// dropped, so a stale URL never surfaces as a 403.
#[derive(Default, Serialize, Deserialize)]
struct Saved {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    streams: BTreeMap<String, Entry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    visitor: Option<Token>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    client: String,
}

#[derive(Clone, Serialize, Deserialize)]
struct Entry {
    #[serde(flatten)]
    stream: Stream,
    expires: u64,
}

#[derive(Clone, Serialize, Deserialize)]
struct Token {
    id: String,
    key: String,
    at: u64,
}

pub(super) struct Cache {
    path: Option<PathBuf>,
    saved: Mutex<Saved>,
}

impl Cache {
    pub(super) fn load(path: Option<PathBuf>) -> Self {
        let saved = path.as_deref().and_then(read).unwrap_or_default();
        Self {
            path,
            saved: Mutex::new(saved),
        }
    }

    pub(super) fn stream(&self, video_id: &str) -> Option<Stream> {
        let saved = mutex::lock(&self.saved);
        let entry = saved.streams.get(video_id)?;
        (now() < entry.expires).then(|| entry.stream.clone())
    }

    pub(super) fn keep(&self, video_id: &str, stream: &Stream) {
        let mut saved = mutex::lock(&self.saved);
        let now = now();
        saved.streams.retain(|_, entry| entry.expires > now);
        saved.streams.insert(
            video_id.to_owned(),
            Entry {
                stream: stream.clone(),
                expires: expiry(&stream.url),
            },
        );
        self.write(&saved);
    }

    /// A URL that just failed is not handed out again: the next play of the
    /// same song would fail exactly the same way, and a signed URL is revoked
    /// long before the expiry it advertises.
    pub(super) fn forget(&self, video_id: &str) {
        let mut saved = mutex::lock(&self.saved);
        if saved.streams.remove(video_id).is_some() {
            self.write(&saved);
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
        (now() < token.at + VISITOR).then(|| Visitor::resumed(&token.id, &token.key))
    }

    pub(super) fn remember_visitor(&self, visitor: &Visitor) {
        let mut saved = mutex::lock(&self.saved);
        saved.visitor = Some(Token {
            id: visitor.id.clone(),
            key: visitor.key.clone(),
            at: now(),
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
        let Some(path) = self.path.as_deref() else {
            return;
        };

        let Ok(text) = serde_json::to_string(saved) else {
            return;
        };

        if let Err(error) = save(path, &text) {
            log::debug!("youtube cache: {error}");
        }
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

fn read(path: &Path) -> Option<Saved> {
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

fn save(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }

    fs::write(path, text)
}

#[cfg(test)]
#[path = "cache.test.rs"]
mod tests;
