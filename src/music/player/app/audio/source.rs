use std::io::{self, Read, Seek, SeekFrom};
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
    pub(super) ahead: mpsc::SyncSender<u64>,
    pub(super) position: u64,
    pub(super) length: u64,
    pub(super) chunk: u64,
}

pub(super) struct Cache {
    pub(super) data: Vec<u8>,
    pub(super) done: Vec<bool>,
    pub(super) error: Option<String>,
    pub(super) chunk: u64,
}

const CACHE_CHUNK: u64 = 512 * 1024;
const MAX_LENGTH: u64 = 512 * 1024 * 1024;

pub(super) fn chunk_range(index: u64, length: u64, chunk: u64) -> (u64, u64) {
    let start = index * chunk;
    let end = ((index + 1) * chunk).min(length) - 1;
    (start, end)
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
    let mut last = String::from("chunk not attempted");
    for attempt in 0..3 {
        let fetched = (|| -> Result<Vec<u8>, String> {
            let response = client
                .get(url)
                .header(reqwest::header::RANGE, format!("bytes={start}-{end}"))
                .header(reqwest::header::USER_AGENT, agent)
                .header(reqwest::header::ACCEPT_ENCODING, "identity")
                .send()
                .map_err(|error| error.to_string())?;
            if response.status() != reqwest::StatusCode::PARTIAL_CONTENT {
                return Err(format!("range answered {}", response.status()));
            }

            let mut body = Vec::with_capacity((end - start + 1) as usize);
            response
                .take(end - start + 1)
                .read_to_end(&mut body)
                .map_err(|error| error.to_string())?;
            if body.len() as u64 != end - start + 1 {
                return Err("short range read".to_owned());
            }

            Ok(body)
        })();
        match fetched {
            Ok(body) => return Ok(body),
            Err(error) => {
                log::warn!("audio: chunk {index} attempt {attempt}: {error}");
                last = error;
                thread::sleep(Duration::from_millis(100 + attempt * 150));
            }
        }
    }

    Err(format!("chunk {index}: {last}"))
}

pub(super) fn ready(cache: &Mutex<Cache>, index: u64) -> bool {
    mutex::lock(cache)
        .done
        .get(index as usize)
        .copied()
        .unwrap_or(true)
}

pub(super) fn store(cache: &Mutex<Cache>, index: u64, body: &[u8]) {
    let mut cache = mutex::lock(cache);
    let offset = index as usize * cache.chunk as usize;
    cache.data[offset..offset + body.len()].copy_from_slice(body);
    cache.done[index as usize] = true;
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

        let response = client
            .get(url)
            .header(reqwest::header::RANGE, "bytes=0-0")
            .header(reqwest::header::USER_AGENT, agent)
            .header(reqwest::header::ACCEPT_ENCODING, "identity")
            .send()
            .map_err(|error| error.to_string())?;

        if !response.status().is_success() {
            return Err(format!("stream answered {}", response.status()));
        }

        let length = response
            .headers()
            .get(reqwest::header::CONTENT_RANGE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.rsplit('/').next())
            .and_then(|value| value.parse::<u64>().ok())
            .or_else(|| response.content_length())
            .ok_or_else(|| "stream length unknown".to_owned())?;
        drop(response);

        if length == 0 {
            return Err("stream is empty".to_owned());
        }

        if length > MAX_LENGTH {
            return Err(format!("stream is too long: {length} bytes"));
        }

        let chunks = length.div_ceil(chunk);
        let cache = Arc::new(Mutex::new(Cache {
            data: vec![0u8; length as usize],
            done: vec![false; chunks as usize],
            error: None,
            chunk,
        }));

        // One chunk ahead of the decoder, so reaching a boundary is a copy and
        // not a wait. Best effort: a busy or failed prefetch costs nothing.
        let (ahead, requests) = mpsc::sync_channel::<u64>(1);
        let worker_cache = cache.clone();
        let worker_client = client.clone();
        let worker_url = url.to_owned();
        let worker_agent = agent.to_owned();
        thread::Builder::new()
            .name("jondo-stream-ahead".to_owned())
            .spawn(move || {
                while let Ok(index) = requests.recv() {
                    if index >= chunks || ready(&worker_cache, index) {
                        continue;
                    }

                    if let Ok(body) = fetch_chunk(
                        &worker_client,
                        &worker_url,
                        &worker_agent,
                        length,
                        index,
                        chunk,
                    ) {
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
            ahead,
            position: 0,
            length,
            chunk,
        })
    }

    /// The bytes of one chunk, asked for when the decoder reaches it. A stream
    /// is played, not downloaded: only the chunks played touch memory.
    pub(super) fn wait(&self, index: usize) -> io::Result<()> {
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
        let _ = self.ahead.try_send(index as u64 + 1);
        Ok(())
    }
}

impl Read for HttpSource {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if self.position >= self.length {
            return Ok(0);
        }

        let index = (self.position / self.chunk) as usize;
        self.wait(index)?;

        let cache = mutex::lock(&self.cache);
        if let Some(error) = &cache.error {
            return Err(io::Error::other(error.clone()));
        }

        let chunk_end = (index as u64 + 1) * self.chunk;
        let end = self
            .length
            .min(self.position + buffer.len() as u64)
            .min(chunk_end);
        let start = self.position as usize;
        let count = (end - self.position) as usize;
        buffer[..count].copy_from_slice(&cache.data[start..start + count]);
        self.position += count as u64;
        Ok(count)
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
