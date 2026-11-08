use std::time::{SystemTime, UNIX_EPOCH};

pub use crate::shared::music::domain::room_id;

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis() as i64)
        .unwrap_or_default()
}

pub fn project(position_ms: f64, at: i64, playing: bool, now: i64) -> f64 {
    let seconds = if playing {
        (position_ms + (now - at) as f64) / 1000.0
    } else {
        position_ms / 1000.0
    };

    seconds.max(0.0)
}

pub fn translate(reason: &str) -> String {
    match reason {
        "host left" => "The host left.".to_owned(),
        "room is full" => "The room is full.".to_owned(),
        "room already has a host" => "This room already has a host.".to_owned(),
        "invalid room password" => "Invalid room password.".to_owned(),
        other => other.to_owned(),
    }
}

pub fn code() -> String {
    use rand::Rng;
    const ALPHABET: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut rng = rand::rng();
    (0..6)
        .map(|_| ALPHABET[rng.random_range(0..ALPHABET.len())] as char)
        .collect()
}

pub fn hex(n: usize) -> String {
    use rand::Rng;
    let mut rng = rand::rng();
    (0..n * 2)
        .map(|_| format!("{:x}", rng.random_range(0..16)))
        .collect()
}
