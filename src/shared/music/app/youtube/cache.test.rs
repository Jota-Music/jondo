use std::fs;
use std::sync::Mutex;

use super::*;
use crate::shared::storage::cache;

/// The session is one file per app, so the tests that read or write it take
/// turns instead of racing on the scratch folder.
static TURN: Mutex<()> = Mutex::new(());

fn signed(seconds: u64) -> String {
    format!("https://rr1.googlevideo.com/videoplayback?expire={seconds}&dur=214.5")
}

fn stream(url: &str) -> Stream {
    Stream {
        url: url.to_owned(),
        agent: CLIENTS[0].agent.to_owned(),
        duration: 214.0,
    }
}

/// A URL resolved in a previous run answers again without a player request,
/// which is the whole point of keeping it.
#[test]
fn a_stream_survives_a_restart() {
    let cache = Cache::load(true);
    cache.keep("restart-abc", &stream(&signed(now() + 6 * 60 * 60)));
    drop(cache);

    let cache = Cache::load(true);
    let (stream, saved) = cache.stream("restart-abc").expect("the stream survived");
    assert_eq!(stream.agent, CLIENTS[0].agent);
    assert_eq!(stream.duration, 214.0);
    assert!(saved > 0, "the envelope carries when it was saved");

    // the file is Jota's audio: the client that resolved it, not the agent
    let text = fs::read_to_string(cache::path(&stream_name("restart-abc")).unwrap()).unwrap();
    let stored: serde_json::Value = serde_json::from_str(&text).unwrap();
    let audio = &stored["value"];
    assert_eq!(audio["clientName"], CLIENTS[0].name);
    assert_eq!(audio["videoId"], "restart-abc");
    assert!(audio["expireAt"].is_i64());
    assert!(audio.get("agent").is_none(), "{text}");
}

/// The URL carries its own expiry, and it is hours away: a flat window would
/// throw away a link that still works.
#[test]
fn the_expiry_comes_from_the_url() {
    let now = now();
    let hours = expiry(&signed(now + 6 * 60 * 60));
    assert!(
        hours >= now + 6 * 60 * 60 - 60,
        "a six hour URL expired after {} seconds",
        hours - now
    );

    let plain = expiry("https://example.com/audio");
    assert!(
        plain >= now + FALLBACK - 60 && plain <= now + FALLBACK + 60,
        "a URL without an expiry got {} seconds",
        plain - now
    );
}

/// A URL that dies before it can be used is not handed out: the player would
/// only fail on it.
#[test]
fn an_expired_url_is_not_handed_out() {
    let cache = Cache::load(false);
    cache.keep("expired-old", &stream(&signed(now().saturating_sub(120))));
    cache.keep("expired-live", &stream(&signed(now() + 3600)));

    assert!(cache.stream("expired-old").is_none());
    assert!(cache.stream("expired-live").is_some());
}

/// The client that last played is remembered, so a session does not walk the
/// list per track.
#[test]
fn the_client_that_played_is_remembered() {
    let _turn = TURN.lock().unwrap_or_else(|error| error.into_inner());
    let _ = cache::forget("youtube/session");

    let cache = Cache::load(true);
    assert!(cache.client().is_none());

    cache.remember_client("IOS");
    drop(cache);

    assert_eq!(Cache::load(true).client().as_deref(), Some("IOS"));
}

/// The visitor token saves the homepage scrape at a cold start, but it is only
/// good for a day. The file carries the token alone, as Jota writes it: the API
/// key is not persisted, so a resumed request uses the well-known one.
#[test]
fn the_visitor_token_is_kept_for_a_day() {
    let _turn = TURN.lock().unwrap_or_else(|error| error.into_inner());
    let _ = cache::forget("youtube/session");

    let cache = Cache::load(true);
    cache.remember_visitor(&Visitor::resumed("visitor-id", "api-key"));
    drop(cache);

    let cache = Cache::load(true);
    let visitor = cache.visitor().expect("the token survived");
    assert_eq!(visitor.id, "visitor-id");
    assert!(visitor.key.is_empty(), "the key is not persisted");

    // the file is Jota's: raw JSON, a token with a deadline, no envelope
    let text = fs::read_to_string(cache::path(super::SESSION).unwrap()).unwrap();
    let stored: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(stored["visitor"]["value"], "visitor-id");
    assert!(stored["visitor"]["at"].is_i64());
    assert!(stored.get("name").is_none(), "{text}");
}

/// A URL revoked before its expiry is dropped for good: keeping it would make
/// every later play of the song fail on the same link.
#[test]
fn a_failed_url_is_forgotten() {
    let cache = Cache::load(true);
    cache.keep("forget-abc", &stream(&signed(now() + 6 * 60 * 60)));
    assert!(cache.stream("forget-abc").is_some());

    cache.forget("forget-abc");
    assert!(cache.stream("forget-abc").is_none());
    drop(cache);

    assert!(Cache::load(true).stream("forget-abc").is_none());
}

/// A stream lives in its own file: keeping one never rewrites another.
#[test]
fn one_file_per_video() {
    let cache = Cache::load(true);
    cache.keep("file-one", &stream(&signed(now() + 6 * 60 * 60)));
    cache.keep("file-two", &stream(&signed(now() + 6 * 60 * 60)));

    let dir = cache::cache_dir().unwrap().join("youtube").join("streams");
    let files = fs::read_dir(dir).unwrap().count();
    assert!(files >= 2, "each video keeps its own entry");
    assert!(cache::path(&stream_name("file-one")).unwrap().exists());
    assert!(cache::path(&stream_name("file-two")).unwrap().exists());
}

/// A half written session must cost a scrape, never a crash.
#[test]
fn a_broken_session_is_empty() {
    let _turn = TURN.lock().unwrap_or_else(|error| error.into_inner());
    let path = cache::path(super::SESSION).unwrap();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, "{not json").unwrap();

    let cache = Cache::load(true);
    assert!(cache.client().is_none());
    assert!(cache.visitor().is_none());

    let _ = fs::remove_file(path);
}
