use std::cell::RefCell;
use std::time::{Duration, Instant};

use gpui::{App, ElementId, Pixels, Point, ScrollEvent, ScrollSource, TouchPhase};

use super::state::*;
/// How long the sheet takes to slide in, and how long the caller keeps it
/// mounted after it starts closing.
use super::*;

pub(super) fn press(id: &ElementId, y: f32) {
    STATES.with(|states| {
        if let Some(state) = states.borrow_mut().get_mut(id) {
            state.down_y = y;
            state.offset = 0.0;
            state.active = false;
            state.track.clear();
            state.settle = None;
            state.dismissed = None;
            state.absorb = false;
        }
    });
}

pub(super) fn begin(id: &ElementId) {
    STATES.with(|states| {
        if let Some(state) = states.borrow_mut().get_mut(id) {
            state.active = true;
            state.offset = 0.0;
            state.track.clear();
            state.track.record(Instant::now(), 0.0);
            state.settle = None;
        }
    });
}

pub(super) fn drag(id: &ElementId, y: f32) {
    STATES.with(|states| {
        let mut states = states.borrow_mut();
        let Some(state) = states.get_mut(id) else {
            return;
        };

        if !state.active {
            return;
        }

        let offset = (y - state.down_y).max(0.0);
        state.offset = offset;
        state.track.record(Instant::now(), offset);
    });
}

pub(super) fn release_sheet(id: &ElementId) -> bool {
    STATES.with(|states| {
        let mut states = states.borrow_mut();
        let Some(state) = states.get_mut(id) else {
            return false;
        };

        if !state.active {
            return false;
        }

        let now = Instant::now();
        state.active = false;
        let from = state.offset.max(0.0);
        let speed = state.track.speed(now);
        state.track.clear();
        state.offset = 0.0;

        if from <= 0.5 {
            state.settle = None;
            return false;
        }

        if dismisses(from, speed) {
            state.dismissed = Some(from);
            state.settle = None;
            true
        } else {
            state.settle = Some(Motion {
                from,
                at: now,
                duration: settle_time(from, speed),
            });
            false
        }
    })
}

pub(super) fn dismisses(offset: f32, velocity: f32) -> bool {
    offset > CLOSE || velocity > FLING
}

/// How long the sheet takes to come back to rest. Constant deceleration from
/// the release speed covers `from` in `2·from / speed`, so a fast hand puts it
/// back at once and a slow one still gets a glide instead of a snap. A release
/// that ended in place gets the full curve: there is no speed to follow, and
/// the distance is what the eye expects to see recovered.
pub(super) fn settle_time(from: f32, speed: f32) -> Duration {
    let speed = speed.abs();
    if speed <= 1.0 {
        return DURATION;
    }

    let seconds = 2.0 * from / speed;
    if !seconds.is_finite() {
        return DURATION;
    }

    Duration::from_secs_f32(seconds.clamp(MIN_SETTLE, DURATION.as_secs_f32()))
}

pub(super) fn active(id: &ElementId) -> bool {
    STATES.with(|states| states.borrow().get(id).is_some_and(|state| state.active))
}

/// Moves an armed sheet by an incremental pan delta. The finger's first movement
/// is reported as a scroll step, so the body drag feeds the same state the
/// grab-strip drag uses and the two cannot disagree.
pub(super) fn shift(id: &ElementId, delta: f32) {
    STATES.with(|states| {
        let mut states = states.borrow_mut();
        let Some(state) = states.get_mut(id) else {
            return;
        };

        if !state.active {
            return;
        }

        state.offset = (state.offset + delta).max(0.0);
        state.track.record(Instant::now(), state.offset);
    });
}

/// Returns a pulled sheet to rest when the system takes the touch away.
pub(super) fn abort(id: &ElementId) {
    STATES.with(|states| {
        let mut states = states.borrow_mut();
        let Some(state) = states.get_mut(id) else {
            return;
        };

        if !state.active {
            return;
        }

        let from = state.offset.max(0.0);
        state.active = false;
        state.track.clear();
        state.offset = 0.0;
        state.settle = if from > 0.5 {
            Some(Motion {
                from,
                at: Instant::now(),
                duration: DURATION,
            })
        } else {
            None
        };
    });
}

pub(super) fn absorb(id: &ElementId, on: bool) {
    STATES.with(|states| {
        if let Some(state) = states.borrow_mut().get_mut(id) {
            state.absorb = on;
        }
    });
}

pub(super) fn absorbing(id: &ElementId) -> bool {
    STATES.with(|states| states.borrow().get(id).is_some_and(|state| state.absorb))
}

/// How long a pan a sheet took keeps its claim after the sheet is gone. The
/// recognizer runs a release into momentum on frames of its own, and every
/// step of that coast carries only where the pan started, so the claim is
/// renewed for as long as the steps keep coming and lapses once they stop.
const KEPT: Duration = Duration::from_secs(1);

thread_local! {
    /// Where the last pan on a sheet layer started. It outlives the sheet on
    /// purpose: the momentum of a dismiss belongs to the sheet that took it,
    /// not to the page the sheet was covering.
    static PAN: RefCell<Option<(Point<Pixels>, Instant)>> = const { RefCell::new(None) };
}

/// Keeps the rest of a pan for the sheet: the page must not scroll with what
/// the release left coasting.
pub(super) fn keep(position: Point<Pixels>) {
    PAN.with(|pan| *pan.borrow_mut() = Some((position, Instant::now())));
}

/// Drops the coasting steps of a pan a sheet already took, before the page's
/// scrollables read them. A page mounts this ahead of its content.
pub fn leftover(event: &ScrollEvent, cx: &mut App) {
    if event.source == ScrollSource::Touch
        && event.touch_phase != TouchPhase::Started
        && kept(event.position)
    {
        cx.stop_propagation();
    }
}

fn kept(position: Point<Pixels>) -> bool {
    PAN.with(|pan| {
        let mut pan = pan.borrow_mut();
        let Some((at, since)) = *pan else {
            return false;
        };

        if at != position || since.elapsed() > KEPT {
            *pan = None;
            return false;
        }

        *pan = Some((at, Instant::now()));
        true
    })
}

pub(super) fn measure(id: &ElementId, height: f32) {
    STATES.with(|states| {
        if let Some(state) = states.borrow_mut().get_mut(id) {
            state.height = height;
        }
    });
}

pub(super) fn progress(at: Instant, duration: Duration, now: Instant) -> f32 {
    (now.duration_since(at).as_secs_f32() / duration.as_secs_f32()).clamp(0.0, 1.0)
}

/// Where the panel sits right now, so a close that interrupts an entrance or a
/// settle starts from what the eye is already seeing.
pub(super) fn visual(state: &State, now: Instant) -> f32 {
    if state.active {
        return state.offset;
    }

    if let Some(motion) = &state.settle {
        let t = progress(motion.at, motion.duration, now);
        return motion.from * (1.0 - snap(t));
    }

    if let Some(opened) = state.opened {
        let t = progress(opened, DURATION, now);
        return state.height * (1.0 - out(t));
    }

    0.0
}

/// Tailwind's `ease-out`, used by the entrance and exit, and the drawer curve
/// a native sheet settles with.
pub(super) fn out(t: f32) -> f32 {
    bezier(t, 0.0, 0.0, 0.2, 1.0)
}

pub(super) fn snap(t: f32) -> f32 {
    bezier(t, 0.32, 0.72, 0.0, 1.0)
}

/// CSS `cubic-bezier` evaluated on x by bisection, which is plenty for a
/// 300ms gesture and needs no interpolation table.
pub(super) fn bezier(x: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    let sample = |t: f32, a: f32, b: f32| {
        3.0 * (1.0 - t).powi(2) * t * a + 3.0 * (1.0 - t) * t * t * b + t * t * t
    };

    let (mut low, mut high) = (0.0, 1.0);
    for _ in 0..8 {
        let middle = (low + high) * 0.5;
        if sample(middle, x1, x2) < x {
            low = middle;
        } else {
            high = middle;
        }
    }

    sample((low + high) * 0.5, y1, y2)
}
