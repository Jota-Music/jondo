use std::time::{Duration, Instant};

use gpui::ElementId;

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
            state.velocity = 0.0;
            state.moved_at = None;
            state.settle = None;
            state.dismissed = None;
        }
    });
}

pub(super) fn begin(id: &ElementId) {
    STATES.with(|states| {
        if let Some(state) = states.borrow_mut().get_mut(id) {
            state.active = true;
            state.offset = 0.0;
            state.velocity = 0.0;
            state.moved_at = Some(Instant::now());
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

        let now = Instant::now();
        let offset = (y - state.down_y).max(0.0);
        if let Some(at) = state.moved_at {
            let dt = now.duration_since(at).as_secs_f32().clamp(0.004, 0.1);
            let instant = (offset - state.offset) / dt;
            state.velocity = state.velocity * 0.35 + instant * 0.65;
        }

        state.offset = offset;
        state.moved_at = Some(now);
    });
}

pub(super) fn release_sheet(id: &ElementId) -> bool {
    STATES.with(|states| {
        let mut states = states.borrow_mut();
        let Some(state) = states.get_mut(id) else {
            return false;
        };

        if !state.active {
            state.moved_at = None;
            return false;
        }

        state.active = false;
        let now = Instant::now();
        let idle = state
            .moved_at
            .map(|at| now.duration_since(at).as_secs_f32())
            .unwrap_or(1.0);
        let velocity = if idle > 0.1 {
            0.0
        } else {
            state.velocity.max(0.0)
        };

        state.velocity = 0.0;
        state.moved_at = None;
        if dismisses(state.offset, velocity) {
            state.dismissed = Some(state.offset);
            state.offset = 0.0;
            true
        } else {
            state.settle = Some(Motion {
                from: state.offset,
                at: now,
            });
            false
        }
    })
}

pub(super) fn dismisses(offset: f32, velocity: f32) -> bool {
    offset > CLOSE || velocity > FLING
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
        let t = progress(motion.at, DURATION, now);
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
