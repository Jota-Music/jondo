use std::io::{self, Read, Seek, SeekFrom};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;

use crate::shared::kernel::app::mutex;
use symphonia::core::io::MediaSource;

pub(super) struct HttpSource {
    pub(super) client: reqwest::blocking::Client,
    pub(super) url: String,
    pub(super) agent: String,
    pub(super) cache: Arc<Mutex<Cache>>,
    /// The chunk the decoder is on, shared with the worker so it fills the
    /// window around the read position and not a chain of single chunks.
    pub(super) at: Arc<AtomicU64>,
    pub(super) awake: mpsc::SyncSender<()>,
    pub(super) position: u64,
    pub(super) length: u64,
    pub(super) chunk: u64,
}

pub(super) struct Cache {
    pub(super) data: Vec<u8>,
    pub(super) slots: Vec<Option<u64>>,
    pub(super) error: Option<String>,
    pub(super) chunk: u64,
}

const CACHE_CHUNK: u64 = 128 * 1024;
/// Chunks held in memory at once. The decoder reads sequentially and the worker
/// keeps the ones ahead of it in memory, so four is slack for a seek; 128 KiB
/// each keeps the whole stream near half a megabyte instead of the size of the
/// file, and keeps the wait before the first byte at 128 KiB instead of half a
/// megabyte.
const CACHE_WINDOW: usize = 4;
const MAX_LENGTH: u64 = 512 * 1024 * 1024;

pub(super) fn chunk_range(index: u64, length: u64, chunk: u64) -> (u64, u64) {
    let start = index * chunk;
    let end = ((index + 1) * chunk).min(length) - 1;
    (start, end)
}

/// Why a range failed. A signed URL that answers "no" answers "no" on the next
/// try too, so it is not worth three round trips and a second of sleep before
/// the player is told to resolve a fresh one.
enum Failure {
    Retry(String),
    Dead(String),
}

/// The statuses a signed URL never recovers from: the signature expired, the
/// video is gone, or the client asking is not the one it was signed for.
fn permanent(status: reqwest::StatusCode) -> bool {
    matches!(status.as_u16(), 401 | 403 | 404 | 410)
}

/// One bounded range, retried. The answer carries the length of the whole
/// stream, so the first chunk is also the request that learns it: opening a
/// stream costs one round trip, not two.
pub(super) fn fetch_range(
    client: &reqwest::blocking::Client,
    url: &str,
    agent: &str,
    start: u64,
    end: u64,
) -> Result<(Vec<u8>, u64), String> {
    let mut last = String::from("range not attempted");
    for attempt in 0..3 {
        let fetched = (|| -> Result<(Vec<u8>, u64), Failure> {
            let response = client
                .get(url)
                .header(reqwest::header::RANGE, format!("bytes={start}-{end}"))
                .header(reqwest::header::USER_AGENT, agent)
                .header(reqwest::header::ACCEPT_ENCODING, "identity")
                .send()
                .map_err(|error| Failure::Retry(error.to_string()))?;

            let status = response.status();
            if status != reqwest::StatusCode::PARTIAL_CONTENT {
                let error = format!("range answered {status}");
                return Err(match permanent(status) {
                    true => Failure::Dead(error),
                    false => Failure::Retry(error),
                });
            }

            let length = response
                .headers()
                .get(reqwest::header::CONTENT_RANGE)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.rsplit('/').next())
                .and_then(|value| value.trim().parse::<u64>().ok())
                .ok_or_else(|| Failure::Dead("stream length unknown".to_owned()))?;

            // The range that opens a short stream asks past its end: the body
            // is what the stream has left, not the size of the request.
            let expected = length.saturating_sub(start).min(end - start + 1);
            let mut body = Vec::with_capacity(expected as usize);
            response
                .take(expected)
                .read_to_end(&mut body)
                .map_err(|error| Failure::Retry(error.to_string()))?;
            if body.len() as u64 != expected {
                return Err(Failure::Retry("short range read".to_owned()));
            }

            Ok((body, length))
        })();
        match fetched {
            Ok(range) => return Ok(range),
            Err(Failure::Dead(error)) => return Err(format!("bytes {start}-{end}: {error}")),
            Err(Failure::Retry(error)) => {
                log::warn!("audio: bytes {start}-{end} attempt {attempt}: {error}");
                last = error;
                thread::sleep(Duration::from_millis(100 + attempt * 150));
            }
        }
    }

    Err(format!("bytes {start}-{end}: {last}"))
}

pub(super) fn fetch_chunk(
    client: &reqwest::blocking::Client,
    url: &str,
    agent: &str,
    length: u64,
    index: u64,
    chunk: u64,
) -> Result<Vec<u8>, String> {
    let (start, end) = chunk_range(index, length, chunk);
    fetch_range(client, url, agent, start, end).map(|(body, _)| body)
}

pub(super) fn ready(cache: &Mutex<Cache>, index: u64) -> bool {
    let cache = mutex::lock(cache);
    let slot = (index % cache.slots.len() as u64) as usize;
    cache.slots[slot] == Some(index)
}

pub(super) fn store(cache: &Mutex<Cache>, index: u64, body: &[u8]) {
    let mut cache = mutex::lock(cache);
    let slot = (index % cache.slots.len() as u64) as usize;
    let offset = slot * cache.chunk as usize;
    cache.data[offset..offset + body.len()].copy_from_slice(body);
    cache.slots[slot] = Some(index);
}

impl HttpSource {
    pub(super) fn new(url: &str, agent: &str) -> Result<Self, String> {
        Self::open(url, agent, CACHE_CHUNK)
    }

    pub(super) fn open(url: &str, agent: &str, chunk: u64) -> Result<Self, String> {
        let client = reqwest::blocking::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|error| error.to_string())?;

        // The first chunk is the opening request: it both proves the stream
        // answers and hands back the length, so nothing is asked twice.
        let (first, length) = fetch_range(&client, url, agent, 0, chunk - 1)?;

        if length == 0 {
            return Err("stream is empty".to_owned());
        }

        if length > MAX_LENGTH {
            return Err(format!("stream is too long: {length} bytes"));
        }

        let chunks = length.div_ceil(chunk);
        let window = (chunks as usize).min(CACHE_WINDOW);
        let cache = Arc::new(Mutex::new(Cache {
            data: vec![0u8; window * chunk as usize],
            slots: vec![None; window],
            error: None,
            chunk,
        }));
        store(&cache, 0, &first);

        // The worker keeps the window ahead of the decoder in memory, so a
        // chunk boundary is a copy and a slow request has the window to land
        // in. Best effort: a busy or failed prefetch costs nothing.
        let (awake, requests) = mpsc::sync_channel::<()>(1);
        let at = Arc::new(AtomicU64::new(0));
        let worker_cache = cache.clone();
        let worker_client = client.clone();
        let worker_url = url.to_owned();
        let worker_agent = agent.to_owned();
        let worker_at = at.clone();
        thread::Builder::new()
            .name("jondo-stream-ahead".to_owned())
            .spawn(move || {
                while requests.recv().is_ok() {
                    // The scan starts from the read position every time, so a
                    // chunk that landed early cannot stop the chain: the next
                    // wake resumes at whatever the decoder is reading now.
                    loop {
                        let position = worker_at.load(Ordering::Relaxed);
                        let Some(index) = (position + 1..position + window as u64)
                            .filter(|index| *index < chunks)
                            .find(|index| !ready(&worker_cache, *index))
                        else {
                            break;
                        };

                        let Ok(body) = fetch_chunk(
                            &worker_client,
                            &worker_url,
                            &worker_agent,
                            length,
                            index,
                            chunk,
                        ) else {
                            break;
                        };

                        store(&worker_cache, index, &body);
                    }
                }
            })
            .ok();

        Ok(Self {
            client,
            url: url.to_owned(),
            agent: agent.to_owned(),
            cache,
            at,
            awake,
            position: 0,
            length,
            chunk,
        })
    }

    /// The bytes of one chunk, asked for when the decoder reaches it. A stream
    /// is played, not downloaded: only the chunks in the window touch memory.
    pub(super) fn wait(&self, index: usize) -> io::Result<()> {
        // Telling the worker where the decoder is wakes it, and the position is
        // what it prefetches around. A wake that lands while the worker is busy
        // is dropped on purpose: the worker rescans when it finishes.
        self.at.store(index as u64, Ordering::Relaxed);
        let _ = self.awake.try_send(());

        if ready(&self.cache, index as u64) {
            return Ok(());
        }

        let error = mutex::lock(&self.cache).error.clone();
        if let Some(error) = error {
            return Err(io::Error::other(error));
        }

        let body = fetch_chunk(
            &self.client,
            &self.url,
            &self.agent,
            self.length,
            index as u64,
            self.chunk,
        )
        .map_err(|error| {
            mutex::lock(&self.cache)
                .error
                .get_or_insert_with(|| error.clone());
            io::Error::other(error)
        })?;
        store(&self.cache, index as u64, &body);
        Ok(())
    }
}

impl Read for HttpSource {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if self.position >= self.length {
            return Ok(0);
        }

        // The prefetch worker may recycle the slot between the wait and the
        // copy, so the slot is checked under the lock that reads it. A retry
        // refetches; it cannot spin, the worker only writes the chunks after
        // this one, which land in other slots.
        for _ in 0..3 {
            let index = self.position / self.chunk;
            self.wait(index as usize)?;

            let cache = mutex::lock(&self.cache);
            if let Some(error) = &cache.error {
                return Err(io::Error::other(error.clone()));
            }

            let slot = (index % cache.slots.len() as u64) as usize;
            if cache.slots[slot] != Some(index) {
                drop(cache);
                continue;
            }

            let chunk_end = (index + 1) * self.chunk;
            let end = self
                .length
                .min(self.position + buffer.len() as u64)
                .min(chunk_end);
            let start = slot * self.chunk as usize + (self.position - index * self.chunk) as usize;
            let count = (end - self.position) as usize;
            buffer[..count].copy_from_slice(&cache.data[start..start + count]);
            self.position += count as u64;
            return Ok(count);
        }

        Err(io::Error::other("stream chunk evicted under the reader"))
    }
}

impl Seek for HttpSource {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        let target = match position {
            SeekFrom::Start(value) => value as i128,
            SeekFrom::Current(delta) => self.position as i128 + delta as i128,
            SeekFrom::End(delta) => self.length as i128 + delta as i128,
        };

        self.position = target.clamp(0, self.length as i128) as u64;
        Ok(self.position)
    }
}

impl MediaSource for HttpSource {
    fn is_seekable(&self) -> bool {
        true
    }

    fn byte_len(&self) -> Option<u64> {
        Some(self.length)
    }
}

#[cfg(test)]
#[path = "source.test.rs"]
mod tests;
