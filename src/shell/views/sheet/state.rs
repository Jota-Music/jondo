use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui::{Context, ElementId, IntoElement, Render, Window, div};

use crate::shell::app::Jondo;

pub(super) type Close = Rc<dyn Fn(&mut Jondo, &mut Window, &mut Context<Jondo>)>;

pub(super) struct Motion {
    pub(super) from: f32,
    pub(super) at: Instant,
    /// How long this motion takes. A release carries the hand's speed, so the
    /// settle covers the distance in the time a constant deceleration from
    /// that speed would: a short pull comes back quickly instead of gliding
    /// for the full curve.
    pub(super) duration: Duration,
}

/// The last touches of a drag, kept to estimate the release velocity.
///
/// The gesture recognizer only hands over positions, so the speed has to be
/// measured here: it decides both whether a short pull is a dismiss and how
/// long the sheet takes to come back. The speed is measured across the most
/// recent window of samples instead of as an average of per-sample deltas,
/// because touch samples arrive at irregular intervals and a per-sample
/// quotient turns that jitter into a speed that swings by hundreds of pixels
/// per second.
const SAMPLES: usize = 24;
/// How far back the release velocity looks. Older samples describe an earlier
/// part of the gesture, not its speed at release.
const WINDOW: Duration = Duration::from_millis(100);
/// A pause longer than this means the finger stopped before lifting, so the
/// gesture ended in place and must not read as a fling.
const STOPPED: Duration = Duration::from_millis(40);
/// Ceiling on a measured speed, matching the recognizer's own fling ceiling.
const MAX_SPEED: f32 = 8000.0;

#[derive(Clone, Copy)]
struct Sample {
    at: Instant,
    offset: f32,
}

pub(super) struct Track {
    samples: [Option<Sample>; SAMPLES],
    next: usize,
}

impl Default for Track {
    fn default() -> Self {
        Self {
            samples: [None; SAMPLES],
            next: 0,
        }
    }
}

impl Track {
    pub(super) fn clear(&mut self) {
        self.samples = [None; SAMPLES];
        self.next = 0;
    }

    pub(super) fn record(&mut self, at: Instant, offset: f32) {
        self.samples[self.next] = Some(Sample { at, offset });
        self.next = (self.next + 1) % SAMPLES;
    }

    /// Pixels per second between the oldest and the newest sample of the
    /// window, in the direction the finger moved. Zero when the hand stopped
    /// before lifting or when there is not enough travel to measure.
    pub(super) fn speed(&self, now: Instant) -> f32 {
        let Some(newest) = self.samples.iter().flatten().max_by_key(|sample| sample.at) else {
            return 0.0;
        };

        if now.saturating_duration_since(newest.at) > STOPPED {
            return 0.0;
        }

        let oldest = self
            .samples
            .iter()
            .flatten()
            .filter(|sample| newest.at.saturating_duration_since(sample.at) <= WINDOW)
            .min_by_key(|sample| sample.at);
        let Some(oldest) = oldest else {
            return 0.0;
        };

        let elapsed = newest.at.saturating_duration_since(oldest.at).as_secs_f32();
        if elapsed < 0.004 {
            return 0.0;
        }

        ((newest.offset - oldest.offset) / elapsed).clamp(-MAX_SPEED, MAX_SPEED)
    }
}

#[derive(Default)]
pub(super) struct State {
    pub(super) height: f32,
    pub(super) down_y: f32,
    pub(super) active: bool,
    /// How far the finger has pulled the sheet. It is painted here directly:
    /// the frame shows the newest sample, which is what a native drawer does.
    /// Extrapolating past the finger adds a lead that overshoots on every
    /// change of speed, and the recognizer already runs ahead of the platform
    /// samples when it can.
    pub(super) offset: f32,
    pub(super) track: Track,
    pub(super) settle: Option<Motion>,
    pub(super) dismissed: Option<f32>,
    pub(super) opened: Option<Instant>,
    pub(super) closing_seen: bool,
    pub(super) focused: bool,
    /// A pan was turned into a sheet drag: its leftover momentum belongs to the
    /// sheet, not to the scrollable body under the finger.
    pub(super) absorb: bool,
}

thread_local! {
    pub(super) static STATES: RefCell<HashMap<ElementId, State>> = RefCell::new(HashMap::new());
}

pub(super) struct Grab;
impl Render for Grab {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}
