use std::fs;

use super::*;

fn home(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("jondo-youtube-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    dir.join("youtube.json")
}

fn signed(seconds: u64) -> String {
    format!("https://rr1.googlevideo.com/videoplayback?expire={seconds}&dur=214.5")
}

fn stream(url: &str) -> Stream {
    Stream {
        url: url.to_owned(),
        agent: "agent".to_owned(),
        duration: 214.5,
    }
}

/// A URL resolved in a previous run answers again without a player request,
/// which is the whole point of keeping it.
#[test]
fn a_stream_survives_a_restart() {
    let path = home("restart");
    let cache = Cache::load(Some(path.clone()));
    cache.keep("abc", &stream(&signed(now() + 6 * 60 * 60)));
    drop(cache);

    let cache = Cache::load(Some(path));
    let stream = cache.stream("abc").expect("the stream survived");
    assert_eq!(stream.agent, "agent");
    assert_eq!(stream.duration, 214.5);
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
    let path = home("expired");
    let cache = Cache::load(Some(path));
    cache.keep("old", &stream(&signed(now().saturating_sub(120))));
    cache.keep("live", &stream(&signed(now() + 3600)));

    assert!(cache.stream("old").is_none());
    assert!(cache.stream("live").is_some());
}

/// The client that last played is remembered, so a session does not walk the
/// list per track.
#[test]
fn the_client_that_played_is_remembered() {
    let path = home("client");
    let cache = Cache::load(Some(path.clone()));
    assert!(cache.client().is_none());

    cache.remember_client("IOS");
    drop(cache);

    assert_eq!(Cache::load(Some(path)).client().as_deref(), Some("IOS"));
}

/// The visitor token saves the homepage scrape at a cold start, but it is only
/// good for a day.
#[test]
fn the_visitor_token_is_kept_for_a_day() {
    let path = home("visitor");
    let cache = Cache::load(Some(path.clone()));
    cache.remember_visitor(&Visitor::resumed("visitor-id", "api-key"));
    drop(cache);

    let cache = Cache::load(Some(path));
    let visitor = cache.visitor().expect("the token survived");
    assert_eq!(visitor.id, "visitor-id");
    assert_eq!(visitor.key, "api-key");
}

/// A URL revoked before its expiry is dropped for good: keeping it would make
/// every later play of the song fail on the same link.
#[test]
fn a_failed_url_is_forgotten() {
    let path = home("forget");
    let cache = Cache::load(Some(path.clone()));
    cache.keep("abc", &stream(&signed(now() + 6 * 60 * 60)));
    assert!(cache.stream("abc").is_some());

    cache.forget("abc");
    assert!(cache.stream("abc").is_none());
    drop(cache);

    assert!(Cache::load(Some(path)).stream("abc").is_none());
}

/// A cache that cannot be written costs a resolve, never the track: the entry
/// still lives in memory for this run.
#[test]
fn an_unwritable_cache_never_breaks_the_play() {
    let path = home("unwritable");
    fs::create_dir_all(path.parent().unwrap().parent().unwrap()).unwrap();
    fs::write(path.parent().unwrap(), "not a folder").unwrap();

    let cache = Cache::load(Some(path));
    cache.keep("abc", &stream(&signed(now() + 3600)));
    assert!(cache.stream("abc").is_some());
}

/// A half written file must cost a resolve, never a crash.
#[test]
fn a_broken_file_is_an_empty_cache() {
    let path = home("broken");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, "{not json").unwrap();

    let cache = Cache::load(Some(path));
    assert!(cache.stream("abc").is_none());
    assert!(cache.client().is_none());
    assert!(cache.visitor().is_none());
}
