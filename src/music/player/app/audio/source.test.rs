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

fn first_read(source: &mut HttpSource) -> usize {
    let mut buffer = [0u8; PART];
    source.read(&mut buffer).expect("first read")
}

/// The decoder cannot read one byte until the whole first chunk has landed:
/// the startup wait grows with the chunk, not with what playback needs.
#[test]
fn the_first_read_waits_for_a_whole_chunk() {
    for chunk in [MB / 8, MB / 4, MB / 2] {
        let server = Server::start(8 * MB, FAST, Duration::ZERO);
        let mut source = open(&server, chunk);

        assert_eq!(first_read(&mut source), PART);

        let served = server.served.load(Ordering::SeqCst);
        let played = first_read(&mut source) as u64;
        assert_eq!(played, PART as u64);
        assert!(
            served >= chunk as u64 - PART as u64,
            "chunk {chunk}: only {served} bytes served before the first read"
        );
        assert!(
            served < chunk as u64 + 16 * PART as u64,
            "chunk {chunk}: {served} bytes served, more than the chunk plus one prefetch"
        );
        println!(
            "chunk {:>4} KB -> first read after {served} bytes",
            chunk / 1024
        );
    }
}

/// A smaller chunk is not free: it costs one request per chunk. The first read
/// is the chunk, and nothing else, so the wait is `chunk / bandwidth`.
#[test]
fn the_optimal_chunk_is_the_one_that_fits_the_bandwidth() {
    let rate = MB as u64;
    for chunk in [64 * 1024, 128 * 1024, 512 * 1024] {
        let server = Server::start(8 * MB, rate, Duration::ZERO);
        let mut source = open(&server, chunk);

        let started = Instant::now();
        first_read(&mut source);
        let wait = started.elapsed();

        let expected = Duration::from_secs_f64(chunk as f64 / rate as f64);
        println!(
            "chunk {:>4} KB -> {:>8.1} ms to first read (chunk/bandwidth {:>6.1} ms)",
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

/// Startup opens the stream twice: one request only to learn the length, then
/// the chunk itself. With a real round trip that is two waits in series.
#[test]
fn the_opening_request_is_a_second_round_trip() {
    let server = Server::start(8 * MB, FAST, Duration::from_millis(150));
    let started = Instant::now();
    let mut source = open(&server, 128 * 1024);
    let opened = started.elapsed();
    first_read(&mut source);
    let playing = started.elapsed();
    let requests = server.requests.load(Ordering::SeqCst);

    println!(
        "open {:>6.1} ms, first read {:>6.1} ms, {requests} requests",
        opened.as_secs_f64() * 1000.0,
        playing.as_secs_f64() * 1000.0
    );

    assert_eq!(requests, 2, "startup opened the stream more than twice");
    assert!(
        playing >= Duration::from_millis(300),
        "startup {playing:?} paid less than two delayed round trips"
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
