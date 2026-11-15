use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use super::*;

const PART: usize = 8 * 1024;
const MB: usize = 1024 * 1024;
const FAST: u64 = 4 * MB as u64;

/// A local range server with a capped bandwidth and an optional delay per
/// request, so startup cost can be measured without the network in between.
struct Server {
    url: String,
    served: Arc<AtomicU64>,
    requests: Arc<AtomicU64>,
    _listener: thread::JoinHandle<()>,
}

impl Server {
    fn start(size: usize, rate: u64, delay: Duration) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("range server");
        let url = format!("http://{}/audio", listener.local_addr().unwrap());
        let served = Arc::new(AtomicU64::new(0));
        let requests = Arc::new(AtomicU64::new(0));
        let body = Arc::new(vec![0x5au8; size]);

        let counter = served.clone();
        let hits = requests.clone();
        let handle = thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                let body = body.clone();
                let counter = counter.clone();
                let hits = hits.clone();
                let total = size as u64;
                thread::spawn(move || {
                    serve(stream, &body, total, rate, delay, &counter, &hits);
                });
            }
        });

        Self {
            url,
            served,
            requests,
            _listener: handle,
        }
    }
}

fn serve(
    mut stream: TcpStream,
    body: &[u8],
    total: u64,
    rate: u64,
    delay: Duration,
    served: &AtomicU64,
    requests: &AtomicU64,
) {
    let mut reader = BufReader::new(match stream.try_clone() {
        Ok(clone) => clone,
        Err(_) => return,
    });

    let mut line = String::new();
    if reader.read_line(&mut line).is_err() {
        return;
    }

    let mut range = None;
    loop {
        line.clear();
        if reader.read_line(&mut line).is_err() || line == "\r\n" || line.is_empty() {
            break;
        }

        let lower = line.to_ascii_lowercase();
        if let Some(value) = lower.trim().strip_prefix("range: bytes=")
            && let Some((start, end)) = value.split_once('-')
            && let (Ok(start), Ok(end)) = (start.parse::<u64>(), end.parse::<u64>())
        {
            range = Some((start, end));
        }
    }

    let (start, end) = range.unwrap_or((0, total - 1));
    let end = end.min(total - 1);
    if start > end {
        return;
    }

    thread::sleep(delay);
    requests.fetch_add(1, Ordering::SeqCst);

    let count = end - start + 1;
    let header = format!(
        "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes {start}-{end}/{total}\r\nContent-Length: {count}\r\nAccept-Ranges: bytes\r\nConnection: close\r\n\r\n"
    );
    if stream.write_all(header.as_bytes()).is_err() {
        return;
    }

    for part in body[start as usize..=end as usize].chunks(PART) {
        if stream.write_all(part).is_err() {
            return;
        }

        served.fetch_add(part.len() as u64, Ordering::SeqCst);
        thread::sleep(Duration::from_secs_f64(part.len() as f64 / rate as f64));
    }
}

fn open(server: &Server, chunk: usize) -> HttpSource {
    HttpSource::open(&server.url, "test", chunk as u64).expect("open")
}

/// A server that refuses every range with one status, like a URL that is spent
/// or a video that is gone.
fn denied(status: &'static str) -> (String, Arc<AtomicU64>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("refusing server");
    let url = format!("http://{}/audio", listener.local_addr().unwrap());
    let requests = Arc::new(AtomicU64::new(0));
    let hits = requests.clone();

    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let hits = hits.clone();
            thread::spawn(move || {
                let Ok(clone) = stream.try_clone() else {
                    return;
                };

                let mut reader = BufReader::new(clone);
                let mut line = String::new();
                loop {
                    line.clear();
                    if reader.read_line(&mut line).is_err() || line == "\r\n" || line.is_empty() {
                        break;
                    }
                }

                hits.fetch_add(1, Ordering::SeqCst);
                let response =
                    format!("HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                let _ = stream.write_all(response.as_bytes());
            });
        }
    });

    (url, requests)
}

/// A 403 is googlevideo's burst limit, not a dead URL: the same range answers
/// again a moment later, so the mismatch is worth three asks and their pause.
#[test]
fn a_throttled_range_is_asked_again() {
    let (url, requests) = denied("403 Forbidden");

    assert!(HttpSource::open(&url, "test", 128 * 1024).is_err());
    assert_eq!(requests.load(Ordering::SeqCst), 3, "a 403 was not retried");
}

/// A video that is gone answers 404: asking again only delays the re-resolve
/// that actually plays the song.
#[test]
fn a_gone_video_is_not_retried() {
    let (url, requests) = denied("404 Not Found");

    assert!(HttpSource::open(&url, "test", 128 * 1024).is_err());
    assert_eq!(requests.load(Ordering::SeqCst), 1, "a 404 was asked again");
}

fn first_read(source: &mut HttpSource) -> usize {
    let mut buffer = [0u8; PART];
    source.read(&mut buffer).expect("first read")
}

/// The buffer is a window around the read position, not the file: a two-hour
/// stream costs the same memory as a short one.
#[test]
fn the_buffer_is_a_window_around_the_read_position() {
    let server = Server::start(64 * MB, FAST, Duration::ZERO);
    let source = open(&server, 128 * 1024);
    let cache = mutex::lock(&source.cache);

    assert_eq!(cache.data.len(), CACHE_WINDOW * 128 * 1024);
    assert_eq!(cache.slots.len(), CACHE_WINDOW);
}

/// Reading past the window recycles slots; the offset arithmetic has to keep
/// handing out the bytes at the read position, not the slot's.
#[test]
fn reads_stay_correct_across_the_window() {
    let chunk = 128 * 1024;
    let server = Server::start(8 * MB, FAST, Duration::ZERO);
    let mut source = open(&server, chunk);

    let mut buffer = vec![0u8; PART];
    let mut total = 0usize;
    while total < CACHE_WINDOW * 3 * chunk {
        let read = source.read(&mut buffer).expect("read");
        assert!(read > 0, "read {total} returned nothing");
        assert!(
            buffer[..read].iter().all(|byte| *byte == 0x5a),
            "wrong bytes at {total}"
        );
        total += read;
    }
}

/// The first byte waits `chunk / bandwidth`, and the chunk is the only thing
/// that wait depends on: nothing else is read before the decoder starts.
#[test]
fn the_first_chunk_is_the_startup_wait() {
    let rate = MB as u64;
    for chunk in [64 * 1024, 128 * 1024, 512 * 1024] {
        let server = Server::start(8 * MB, rate, Duration::ZERO);
        let started = Instant::now();
        let mut source = open(&server, chunk);
        let wait = started.elapsed();
        first_read(&mut source);

        let expected = Duration::from_secs_f64(chunk as f64 / rate as f64);
        println!(
            "chunk {:>4} KB -> {:>8.1} ms to first byte (chunk/bandwidth {:>6.1} ms)",
            chunk / 1024,
            wait.as_secs_f64() * 1000.0,
            expected.as_secs_f64() * 1000.0
        );

        assert!(
            wait >= expected.mul_f64(0.7),
            "chunk {}: {wait:?} is faster than the transfer itself",
            chunk / 1024
        );
    }
}

/// Startup pays one round trip: the first chunk is the opening request and its
/// answer carries the length, so no second request is spent learning it.
#[test]
fn the_opening_request_fetches_the_first_chunk() {
    let chunk = 128 * 1024;
    let server = Server::start(8 * MB, FAST, Duration::from_millis(150));
    let started = Instant::now();
    let mut source = open(&server, chunk);
    let opened = started.elapsed();
    let requests = server.requests.load(Ordering::SeqCst);

    println!(
        "open {:>6.1} ms, {requests} request, {} bytes",
        opened.as_secs_f64() * 1000.0,
        server.served.load(Ordering::SeqCst)
    );

    assert_eq!(requests, 1, "startup opened the stream more than once");
    assert!(
        opened >= Duration::from_millis(150),
        "startup {opened:?} skipped the round trip"
    );
    assert!(server.served.load(Ordering::SeqCst) >= chunk as u64);
    assert_eq!(first_read(&mut source), PART);
}

/// The worker fills the window around the read position and keeps it filled as
/// the decoder moves: a chunk that arrived early must not stop the chain, or
/// every other chunk is fetched while the decoder waits.
#[test]
fn the_reader_keeps_the_window_ahead_in_memory() {
    let chunk = 128 * 1024;
    let delay = Duration::from_millis(100);
    let server = Server::start(8 * MB, FAST * 4, delay);
    let mut source = open(&server, chunk);

    first_read(&mut source);
    thread::sleep(delay * 8);
    let ahead = (1..CACHE_WINDOW as u64)
        .filter(|index| ready(&source.cache, *index))
        .count();
    println!("{ahead} chunks ahead of the decoder after one read");
    assert_eq!(
        ahead,
        CACHE_WINDOW - 1,
        "the worker did not fill the window ahead"
    );

    let mut buffer = vec![0u8; PART];
    while source.position < chunk as u64 + PART as u64 {
        let read = source.read(&mut buffer).expect("read");
        assert!(read > 0, "stream ended before the second chunk");
    }
    thread::sleep(delay * 8);

    assert!(
        ready(&source.cache, CACHE_WINDOW as u64),
        "the window stopped moving with the decoder"
    );
}

/// The whole path, as the player runs it: resolve the stream on YouTube, then
/// wait for the first decoded packet. Ignored because it needs the network.
#[test]
#[ignore = "hits youtube.com"]
fn the_real_startup_is_measured_in_round_trips() {
    use crate::shared::youtube::Youtube;

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("tokio");

    let youtube = Youtube::new();
    let video = std::env::var("JONDO_BENCH_VIDEO").unwrap_or_else(|_| "dQw4w9WgXcQ".to_owned());

    for round in 0..3 {
        let started = Instant::now();
        let stream = runtime
            .block_on(youtube.stream(&video))
            .expect("stream resolves");
        let resolved = started.elapsed();

        let mut source = HttpSource::new(&stream.url, &stream.agent).expect("open");
        let opened = started.elapsed();
        first_read(&mut source);
        let playing = started.elapsed();

        println!(
            "round {round}: resolve {:>5.0} ms | open {:>4.0} ms | first bytes {:>4.0} ms | total {:>5.0} ms",
            resolved.as_secs_f64() * 1000.0,
            (opened - resolved).as_secs_f64() * 1000.0,
            (playing - opened).as_secs_f64() * 1000.0,
            playing.as_secs_f64() * 1000.0
        );
    }

    let query = "bohemian rhapsody queen";
    let started = Instant::now();
    let found = runtime.block_on(youtube.search(query)).expect("search");
    println!(
        "search ({} hits): {:.0} ms",
        found.len(),
        started.elapsed().as_secs_f64() * 1000.0
    );
}
