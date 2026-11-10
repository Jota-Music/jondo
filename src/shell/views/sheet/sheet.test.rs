use std::time::{Duration, Instant};

use gpui::ElementId;

use super::drag::*;
use super::state::*;
use super::{CLOSE, MIN_SETTLE};

#[test]
fn drawer_curve_starts_and_ends_where_it_should() {
    assert!(bezier(0.0, 0.32, 0.72, 0.0, 1.0).abs() < 0.01);
    assert!(bezier(1.0, 0.32, 0.72, 0.0, 1.0) > 0.99);
    assert!(bezier(0.5, 0.32, 0.72, 0.0, 1.0) > 0.5);
}

#[test]
fn a_short_drag_dismisses_when_flung() {
    assert!(!dismisses(40.0, 100.0));
    assert!(dismisses(40.0, 600.0));
    assert!(dismisses(100.0, 10.0));
    // The threshold is the distance, so a fling that pushed the sheet further
    // than the close line dismisses even when the hand stopped before lifting.
    assert!(dismisses(CLOSE + 1.0, 0.0));
}

fn install(id: &ElementId) {
    STATES.with(|states| {
        states.borrow_mut().insert(id.clone(), State::default());
    });
}

#[test]
fn a_touch_that_does_not_move_settles_where_it_started() {
    let id = ElementId::Name("sheet-tap-test".into());
    install(&id);
    press(&id, 100.0);
    begin(&id);
    assert!(active(&id));

    assert!(!release_sheet(&id));
    assert!(!active(&id));
    STATES.with(|states| {
        let states = states.borrow();
        let state = states.get(&id).expect("the sheet state is kept");
        assert!(state.settle.is_none(), "a tap must not run a settle");
        assert_eq!(state.offset, 0.0);
    });
}

#[test]
fn a_drag_past_the_close_line_dismisses() {
    let id = ElementId::Name("sheet-drag-test".into());
    install(&id);
    press(&id, 100.0);
    begin(&id);
    drag(&id, 100.0 + CLOSE + 24.0);

    assert!(release_sheet(&id));
}

#[test]
fn a_pan_shift_adds_to_the_offset_and_never_pulls_up() {
    let id = ElementId::Name("sheet-shift-test".into());
    install(&id);
    press(&id, 100.0);
    begin(&id);

    shift(&id, 40.0);
    shift(&id, 30.0);
    STATES.with(|states| {
        assert_eq!(states.borrow()[&id].offset, 70.0);
    });

    shift(&id, -100.0);
    STATES.with(|states| {
        assert_eq!(states.borrow()[&id].offset, 0.0);
    });
}

#[test]
fn a_steady_swipe_reports_its_speed() {
    let start = Instant::now();
    let mut track = Track::default();
    track.record(start, 0.0);
    track.record(start + Duration::from_millis(16), 10.0);
    track.record(start + Duration::from_millis(32), 20.0);

    let speed = track.speed(start + Duration::from_millis(32));
    assert!(
        (speed - 625.0).abs() < 1.0,
        "20px in 32ms is 625px/s, got {speed}"
    );
}

#[test]
fn an_irregular_sample_rate_does_not_skew_the_speed() {
    // A hand at a constant 500px/s, sampled at irregular intervals with one
    // sample a little early. The two middle steps on their own read 250px/s and
    // 625px/s: a speed built from per-sample deltas would swing between them,
    // which is what made the sheet wobble under the finger. Across the window
    // the gesture still reads as what it was.
    let start = Instant::now();
    let mut track = Track::default();
    for (ms, px) in [(0u64, 0.0), (8, 4.0), (16, 6.0), (24, 12.0), (32, 16.0)] {
        track.record(start + Duration::from_millis(ms), px);
    }

    let speed = track.speed(start + Duration::from_millis(32));
    assert!(
        (speed - 500.0).abs() < 1.0,
        "a hand at 500px/s must read as 500px/s, got {speed}"
    );
}

#[test]
fn a_finger_that_stopped_before_lifting_is_not_a_fling() {
    let start = Instant::now();
    let mut track = Track::default();
    track.record(start, 0.0);
    track.record(start + Duration::from_millis(16), 60.0);

    assert!(
        track.speed(start + Duration::from_millis(120)) == 0.0,
        "a touch that sat still before the lift has no release speed"
    );
}

#[test]
fn a_fast_release_settles_sooner_than_a_frozen_one() {
    let fast = settle_time(80.0, 800.0);
    let slow = settle_time(80.0, 0.0);

    assert!(fast < slow, "the hand's speed must shorten the settle");
    assert!(
        (fast.as_secs_f32() - 0.2).abs() < 0.01,
        "80px at 800px/s decelerates to rest in 0.2s, got {:?}",
        fast
    );
    assert_eq!(slow, super::DURATION);
    assert!(
        fast.as_secs_f32() >= MIN_SETTLE,
        "the settle never becomes an instant snap"
    );
}

#[test]
fn a_cancelled_pan_settles_back() {
    let id = ElementId::Name("sheet-cancel-test".into());
    install(&id);
    press(&id, 100.0);
    begin(&id);
    shift(&id, 40.0);

    abort(&id);
    STATES.with(|states| {
        let states = states.borrow();
        let state = &states[&id];
        assert!(!state.active);
        assert_eq!(state.offset, 0.0);
        assert_eq!(state.settle.as_ref().map(|motion| motion.from), Some(40.0));
    });
}
