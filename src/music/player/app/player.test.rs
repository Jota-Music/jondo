use super::*;

#[test]
fn settle_keeps_the_loader_until_the_engine_plays() {
    let mut player = Player::new(1.0, false);
    assert!(!player.settle());

    player.loading = true;
    assert!(!player.settle());
    assert!(player.loading);
}

#[test]
fn stop_clears_the_loader() {
    let mut player = Player::new(1.0, false);
    player.loading = true;
    player.stop();
    assert!(!player.loading);
}

#[test]
fn a_load_gets_a_single_retry() {
    let mut player = Player::new(1.0, false);
    assert!(player.retry());
    assert!(!player.retry());
}

/// A stream that stops moving is dead, not buffering: the queue must be told
/// so it can resolve a fresh URL instead of leaving the bar frozen forever.
#[test]
fn a_position_that_stops_moving_is_a_stall() {
    let start = Instant::now();
    let mut still = Still::default();

    assert!(!still.froze(10.0, start));
    assert!(!still.froze(10.0, start + Duration::from_secs(1)));
    assert!(still.froze(10.0, start + STALL));
}

/// Real playback moves a little on every tick, far less than the threshold, so
/// the watchdog must not count a slow advance as a stall.
#[test]
fn a_playing_position_is_never_a_stall() {
    let start = Instant::now();
    let mut still = Still::default();

    for tick in 0..300u32 {
        let at = start + Duration::from_millis(16 * u64::from(tick));
        let position = 10.0 + 0.016 * f64::from(tick);
        assert!(!still.froze(position, at), "stalled at tick {tick}");
    }
}

/// Seeking backwards is movement, not silence.
#[test]
fn a_seek_restarts_the_watchdog() {
    let start = Instant::now();
    let mut still = Still::default();

    assert!(!still.froze(30.0, start));
    assert!(!still.froze(5.0, start + Duration::from_secs(3)));
    assert!(still.froze(5.0, start + Duration::from_secs(3) + STALL));
}

fn queued() -> Player {
    let mut player = Player::new(1.0, false);
    player.queue.replace(
        vec![Song {
            id: "local:one".to_owned(),
            name: "One".to_owned(),
            ..Default::default()
        }],
        0,
    );
    player
}

#[test]
fn pausing_a_stream_that_is_still_resolving_keeps_loading_it() {
    let mut player = queued();
    player.loading = true;

    assert!(player.toggle(), "the pause is handled");
    assert!(player.loading, "the stream keeps loading");
    assert!(!player.busy(), "the interface shows a paused track");

    assert!(player.toggle(), "the play intent is handled");
    assert!(!player.paused);
    assert!(player.busy(), "the spinner comes back while it resolves");
}

#[test]
fn stop_drops_a_paused_load() {
    let mut player = queued();
    player.loading = true;
    player.toggle();
    player.stop();
    assert!(!player.paused);
    assert!(!player.loading);
    assert!(!player.busy());
}
