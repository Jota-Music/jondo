//! Keeps a touch that lands during a fling from inheriting the fling's axis,
//! and from turning into a tap.
//!
//! GPUI's touch recognizer stops scroll momentum when a new contact begins
//! and treats that contact as a pan from its first pixel — but, up to
//! gpui-pre 0.3.5, it also locks the contact to the axis of the fling it
//! stopped. A quick sideways swipe (across a wide table, or just across text:
//! momentum starts on any fast release) followed by a swipe up therefore
//! produced only horizontal deltas, and the page looked frozen until the
//! finger lifted. The fix belongs in the recognizer (zed#64239: the caught
//! fling's axis is the new contact's own); this guard is what the platform
//! can do about it until that reaches a release.
//!
//! The same catch also swallows control drags: a contact that catches a
//! fling is never offered as a touch drag, so a scrollbar thumb — visible only
//! while its content scrolls and coasts — could not be grabbed until the
//! coasting had been stopped by an earlier touch.
//!
//! The recognizer takes its momentum on *every* `Started`, and a contact that
//! is cancelled while still pending emits nothing. So before a real contact
//! that may land on momentum, the guard relays a synthetic contact that begins
//! and is cancelled at the same point: it stops the momentum (a zero-delta
//! `Started`/`Cancelled` scroll pair the consumers treat as a no-op), and the
//! real contact then begins on an idle recognizer, which offers it as a drag
//! and otherwise decides its axis from the contact's own first movement past
//! the slop. When no momentum is left the synthetic pair is a no-op (a pending
//! contact cancelled emits nothing).
//!
//! Starting the real contact on an idle recognizer would also turn the touch
//! that stopped the fling into a tap. To keep the recognizer's no-tap rule, a
//! contact that lands on momentum and lifts without moving is relayed as
//! cancelled instead of ended.
//!
//! The recognizer's momentum is not visible from the platform, so the guard
//! estimates it from the release speed and the platform's scroll physics.
//! Genuine taps after the coast has stopped are left alone.
//!
//! Main thread only, like everything in GPUI.

use std::{
    collections::{HashMap, VecDeque},
    time::{Duration, Instant},
};

use gpui::{px, GestureTuning, Pixels, Point, ScrollPhysics, TouchEvent, TouchId, TouchPhase};

/// The recognizer's touch slop: a contact that moved past it panned and may
/// have flung on release; one that did not was a tap or a long press.
const PAN_SLOP: Pixels = px(8.);

/// How far back the release-speed estimate looks, matching the recognizer.
const VELOCITY_WINDOW: Duration = Duration::from_millis(100);

/// A pause before the last movement longer than this means the finger stopped:
/// nothing flings.
const ASSUMED_STOPPED: Duration = Duration::from_millis(40);

const VELOCITY_MAX_SAMPLES: usize = 20;

/// The recognizer's velocity ceiling, matching Flutter's `kMaxFlingVelocity`.
const MAX_FLING_VELOCITY: f32 = 8000.;

/// Momentum below this speed is imperceptible, matching the recognizer.
const MOMENTUM_STOP_VELOCITY: f32 = 10.;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Physics {
    Exponential { decay_per_ms: f32 },
    FrictionSpline { friction: f32, pixels_per_inch: f32 },
}

impl From<ScrollPhysics> for Physics {
    fn from(physics: ScrollPhysics) -> Self {
        match physics {
            ScrollPhysics::Exponential { decay_per_ms } => Self::Exponential { decay_per_ms },
            ScrollPhysics::FrictionSpline {
                friction,
                pixels_per_inch,
            } => Self::FrictionSpline {
                friction,
                pixels_per_inch,
            },
        }
    }
}

impl Physics {
    /// How long a fling released at `speed` coasts, mirroring the recognizer's
    /// private `ScrollPhysics::fling_duration`.
    fn coast(self, speed: f32) -> Duration {
        match self {
            Self::Exponential { decay_per_ms } => {
                if speed <= MOMENTUM_STOP_VELOCITY {
                    return Duration::ZERO;
                }
                let milliseconds = (MOMENTUM_STOP_VELOCITY / speed).ln() / decay_per_ms.ln();
                Duration::from_secs_f32(milliseconds / 1000.)
            }
            Self::FrictionSpline {
                friction,
                pixels_per_inch,
            } => {
                if speed <= 0. {
                    return Duration::ZERO;
                }
                let physical = 9.80665 * 39.37 * pixels_per_inch as f64 * 0.84;
                let deceleration = (0.35 * speed as f64 / (friction as f64 * physical)).ln();
                let rate = 0.78f64.ln() / 0.9f64.ln();
                Duration::from_secs_f64((deceleration / (rate - 1.)).exp())
            }
        }
    }
}

/// Relays raw contacts to GPUI, disarming a possible fling before a contact
/// that would otherwise catch it. `next_id` allocates synthetic contact IDs
/// from the same sequence as the platform's real ones.
pub(crate) struct FlingGuard {
    tuning: GestureTuning,
    physics: Physics,
    /// Where each live contact began, and whether it has moved past the slop.
    contacts: HashMap<TouchId, Contact>,
    /// When the current fling coasts to a stop.
    momentum_until: Option<Instant>,
}

struct Contact {
    start: Point<Pixels>,
    panned: bool,
    /// The contact began while a fling was still coasting: lifting it without
    /// moving stops the content, it does not activate what is under the finger.
    catching: bool,
    samples: Samples,
}

#[derive(Default)]
struct Samples {
    points: VecDeque<(Instant, Point<Pixels>)>,
}

impl Samples {
    fn push(&mut self, time: Instant, position: Point<Pixels>) {
        self.points.push_back((time, position));
        while self.points.len() > VELOCITY_MAX_SAMPLES {
            self.points.pop_front();
        }
    }

    /// The speed at release in pixels per second, or `None` when the finger
    /// had already stopped before lifting.
    fn speed(&self, now: Instant) -> Option<f32> {
        let (newest_time, newest) = *self.points.back()?;
        if now.duration_since(newest_time) > ASSUMED_STOPPED {
            return None;
        }

        let mut previous = newest_time;
        let mut oldest = (newest_time, newest);
        for (time, position) in self.points.iter().rev().skip(1) {
            if newest_time.duration_since(*time) > VELOCITY_WINDOW
                || previous.duration_since(*time) > ASSUMED_STOPPED
            {
                break;
            }
            previous = *time;
            oldest = (*time, *position);
        }

        let elapsed = newest_time.duration_since(oldest.0).as_secs_f32();
        if elapsed <= 0. {
            return None;
        }

        let dx = f32::from(newest.x - oldest.1.x);
        let dy = f32::from(newest.y - oldest.1.y);
        Some((dx * dx + dy * dy).sqrt() / elapsed)
    }
}

impl FlingGuard {
    pub(crate) fn new(tuning: GestureTuning) -> Self {
        Self {
            tuning,
            physics: tuning.scroll_physics.into(),
            contacts: HashMap::new(),
            momentum_until: None,
        }
    }

    /// Relays `event`, preceded by a synthetic contact when it begins while
    /// momentum may still be running.
    pub(crate) fn relay(
        &mut self,
        event: TouchEvent,
        next_id: impl FnOnce() -> TouchId,
        mut emit: impl FnMut(TouchEvent),
    ) {
        self.relay_at(event, Instant::now(), next_id, &mut emit);
    }

    fn relay_at(
        &mut self,
        event: TouchEvent,
        now: Instant,
        next_id: impl FnOnce() -> TouchId,
        emit: &mut impl FnMut(TouchEvent),
    ) {
        match event.phase {
            TouchPhase::Started => {
                let catching = self.momentum_until.take().is_some_and(|until| now < until);
                if catching {
                    let id = next_id();
                    for phase in [TouchPhase::Started, TouchPhase::Cancelled] {
                        emit(TouchEvent {
                            id,
                            phase,
                            position: event.position,
                            ..Default::default()
                        });
                    }
                }
                let mut samples = Samples::default();
                samples.push(now, event.position);
                self.contacts.insert(
                    event.id,
                    Contact {
                        start: event.position,
                        panned: false,
                        catching,
                        samples,
                    },
                );
                emit(event);
            }
            TouchPhase::Moved => {
                if let Some(contact) = self.contacts.get_mut(&event.id) {
                    contact.samples.push(now, event.position);
                    if !contact.panned {
                        let travelled = event.position - contact.start;
                        contact.panned = travelled.magnitude() > f64::from(PAN_SLOP);
                    }
                }
                emit(event);
            }
            TouchPhase::Ended => {
                if let Some(contact) = self.contacts.remove(&event.id) {
                    if contact.catching && !contact.panned {
                        // The recognizer would synthesize a tap here. A touch
                        // that stopped a fling is not an activation.
                        emit(TouchEvent {
                            phase: TouchPhase::Cancelled,
                            ..event
                        });
                        return;
                    }

                    if contact.panned {
                        let fling = contact
                            .samples
                            .speed(now)
                            .filter(|speed| *speed >= self.tuning.min_fling_velocity);
                        if let Some(speed) = fling {
                            let speed = speed.min(MAX_FLING_VELOCITY);
                            self.momentum_until = Some(now + self.physics.coast(speed));
                        }
                    }
                }
                emit(event);
            }
            // A cancelled pan never flings.
            TouchPhase::Cancelled => {
                self.contacts.remove(&event.id);
                emit(event);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::point;

    fn touch(id: u64, phase: TouchPhase, x: f32, y: f32) -> TouchEvent {
        TouchEvent {
            id: TouchId(id),
            phase,
            position: point(px(x), px(y)),
            predicted_position: None,
            force: None,
        }
    }

    fn ms(milliseconds: u64) -> Duration {
        Duration::from_millis(milliseconds)
    }

    /// Relays `(time, event)` pairs at their times, returning what reached GPUI
    /// as `(id, phase, position)`.
    fn relay_all(
        guard: &mut FlingGuard,
        events: impl IntoIterator<Item = (Instant, TouchEvent)>,
    ) -> Vec<(u64, TouchPhase, Point<Pixels>)> {
        let mut out = Vec::new();
        let mut next_id = 100;
        for (now, event) in events {
            guard.relay_at(
                event,
                now,
                || {
                    next_id += 1;
                    TouchId(next_id)
                },
                &mut |event| out.push((event.id.0, event.phase, event.position)),
            );
        }
        out
    }

    /// A fast swipe released with velocity: the recognizer is coasting.
    fn fling(guard: &mut FlingGuard, start: Instant) {
        let relayed = relay_all(
            guard,
            [
                (start, touch(1, TouchPhase::Started, 300., 300.)),
                (start + ms(16), touch(1, TouchPhase::Moved, 240., 300.)),
                (start + ms(32), touch(1, TouchPhase::Moved, 180., 300.)),
                (start + ms(48), touch(1, TouchPhase::Ended, 180., 300.)),
            ],
        );
        assert_eq!(relayed.len(), 4, "a fling is relayed as is");
    }

    /// A slow drag released without speed: no momentum follows.
    fn slow(guard: &mut FlingGuard, start: Instant) {
        relay_all(
            guard,
            [
                (start, touch(1, TouchPhase::Started, 300., 300.)),
                (start + ms(300), touch(1, TouchPhase::Moved, 288., 300.)),
                (start + ms(310), touch(1, TouchPhase::Ended, 288., 300.)),
            ],
        );
    }

    #[test]
    fn a_contact_after_a_fling_is_preceded_by_a_cancelled_contact() {
        let mut guard = FlingGuard::new(GestureTuning::default());
        let now = Instant::now();
        fling(&mut guard, now);

        let relayed = relay_all(
            &mut guard,
            [(now + ms(200), touch(2, TouchPhase::Started, 200., 300.))],
        );

        assert_eq!(
            relayed,
            [
                (101, TouchPhase::Started, point(px(200.), px(300.))),
                (101, TouchPhase::Cancelled, point(px(200.), px(300.))),
                (2, TouchPhase::Started, point(px(200.), px(300.))),
            ]
        );
    }

    #[test]
    fn a_contact_that_stops_a_fling_and_lifts_is_not_a_tap() {
        let mut guard = FlingGuard::new(GestureTuning::default());
        let now = Instant::now();
        fling(&mut guard, now);

        let relayed = relay_all(
            &mut guard,
            [
                (now + ms(200), touch(2, TouchPhase::Started, 200., 300.)),
                (now + ms(240), touch(2, TouchPhase::Ended, 200., 300.)),
            ],
        );

        assert_eq!(
            relayed.last().map(|(_, phase, _)| *phase),
            Some(TouchPhase::Cancelled),
            "stopping a fling must not synthesize a tap"
        );
    }

    #[test]
    fn a_contact_that_stops_a_fling_and_pans_still_flings() {
        let mut guard = FlingGuard::new(GestureTuning::default());
        let now = Instant::now();
        fling(&mut guard, now);

        let relayed = relay_all(
            &mut guard,
            [
                (now + ms(200), touch(2, TouchPhase::Started, 200., 300.)),
                (now + ms(216), touch(2, TouchPhase::Moved, 200., 240.)),
                (now + ms(232), touch(2, TouchPhase::Ended, 200., 180.)),
            ],
        );

        assert_eq!(
            relayed.last().map(|(_, phase, _)| *phase),
            Some(TouchPhase::Ended),
            "a new swipe keeps its own release"
        );
    }

    #[test]
    fn the_guard_is_armed_once_per_fling() {
        let mut guard = FlingGuard::new(GestureTuning::default());
        let now = Instant::now();
        fling(&mut guard, now);

        let first = relay_all(
            &mut guard,
            [
                (now + ms(200), touch(2, TouchPhase::Started, 200., 300.)),
                (now + ms(240), touch(2, TouchPhase::Ended, 200., 300.)),
            ],
        );
        let second = relay_all(
            &mut guard,
            [(now + ms(300), touch(3, TouchPhase::Started, 200., 300.))],
        );

        assert_eq!(first.len(), 4);
        assert_eq!(
            second,
            [(3, TouchPhase::Started, point(px(200.), px(300.)))]
        );
    }

    #[test]
    fn a_contact_after_a_slow_release_is_relayed_as_is() {
        let mut guard = FlingGuard::new(GestureTuning::default());
        let now = Instant::now();
        slow(&mut guard, now);

        let relayed = relay_all(
            &mut guard,
            [(now + ms(400), touch(2, TouchPhase::Started, 200., 300.))],
        );

        assert_eq!(
            relayed,
            [(2, TouchPhase::Started, point(px(200.), px(300.)))]
        );
    }

    #[test]
    fn a_contact_after_the_fling_stops_is_relayed_as_is() {
        let mut guard = FlingGuard::new(GestureTuning::default());
        let now = Instant::now();
        fling(&mut guard, now);

        let relayed = relay_all(
            &mut guard,
            [(now + ms(4000), touch(2, TouchPhase::Started, 200., 300.))],
        );

        assert_eq!(
            relayed,
            [(2, TouchPhase::Started, point(px(200.), px(300.)))]
        );
    }

    #[test]
    fn a_tap_does_not_arm_the_guard() {
        let mut guard = FlingGuard::new(GestureTuning::default());
        let now = Instant::now();
        relay_all(
            &mut guard,
            [
                (now, touch(1, TouchPhase::Started, 300., 300.)),
                (now + ms(16), touch(1, TouchPhase::Moved, 303., 302.)),
                (now + ms(32), touch(1, TouchPhase::Ended, 303., 302.)),
            ],
        );

        let relayed = relay_all(
            &mut guard,
            [(now + ms(100), touch(2, TouchPhase::Started, 300., 300.))],
        );

        assert_eq!(
            relayed,
            [(2, TouchPhase::Started, point(px(300.), px(300.)))]
        );
    }

    #[test]
    fn a_cancelled_pan_does_not_arm_the_guard() {
        let mut guard = FlingGuard::new(GestureTuning::default());
        let now = Instant::now();
        relay_all(
            &mut guard,
            [
                (now, touch(1, TouchPhase::Started, 300., 300.)),
                (now + ms(16), touch(1, TouchPhase::Moved, 200., 300.)),
                (now + ms(32), touch(1, TouchPhase::Cancelled, 200., 300.)),
            ],
        );

        let relayed = relay_all(
            &mut guard,
            [(now + ms(100), touch(2, TouchPhase::Started, 300., 300.))],
        );

        assert_eq!(
            relayed,
            [(2, TouchPhase::Started, point(px(300.), px(300.)))]
        );
    }

    /// The bug this guard must not reintroduce: a touch that stops a coasting
    /// list is not a tap on whatever row it landed over.
    #[gpui::test]
    fn a_contact_that_stops_a_fling_does_not_click(cx: &mut gpui::TestAppContext) {
        use gpui::{
            div, Context, InteractiveElement, IntoElement, Render, StatefulInteractiveElement,
            Styled, Window,
        };
        use std::{cell::Cell, rc::Rc};

        struct Clicker {
            clicks: Rc<Cell<usize>>,
        }

        impl Render for Clicker {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                let clicks = self.clicks.clone();
                div()
                    .id("clicker")
                    .size_full()
                    .on_click(move |_, _, _| clicks.set(clicks.get() + 1))
            }
        }

        let clicks = Rc::new(Cell::new(0));
        let (_, cx) = cx.add_window_view({
            let clicks = clicks.clone();
            move |_, _| Clicker { clicks }
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));

        let mut guard = FlingGuard::new(GestureTuning::default());
        let mut next_id = 100;
        let mut clock = Instant::now();
        let mut relay = |event: TouchEvent, cx: &mut gpui::VisualTestContext| {
            guard.relay_at(
                event,
                clock,
                || {
                    next_id += 1;
                    TouchId(next_id)
                },
                &mut |event| cx.simulate_event(event),
            );
            clock += ms(16);
        };

        relay(touch(1, TouchPhase::Started, 50., 400.), cx);
        relay(touch(1, TouchPhase::Ended, 50., 400.), cx);
        assert_eq!(clicks.get(), 1, "a tap clicks");

        relay(touch(2, TouchPhase::Started, 50., 400.), cx);
        for step in 1..=6 {
            relay(
                touch(2, TouchPhase::Moved, 50., 400. - 40. * step as f32),
                cx,
            );
        }
        relay(touch(2, TouchPhase::Ended, 50., 160.), cx);
        assert_eq!(clicks.get(), 1, "a fling does not click");

        relay(touch(3, TouchPhase::Started, 95., 20.), cx);
        assert_eq!(
            clicks.get(),
            1,
            "stopping a fling does not click on contact"
        );
        relay(touch(3, TouchPhase::Ended, 95., 20.), cx);
        assert_eq!(
            clicks.get(),
            1,
            "a contact that stops the fling must not click the row under it"
        );
    }

    /// The guard forwards a pan unchanged, so the recognizer's momentum must
    /// survive it: a release keeps the list coasting.
    #[gpui::test]
    fn a_guarded_release_keeps_its_inertia(cx: &mut gpui::TestAppContext) {
        use gpui::{
            div, Context, InteractiveElement, IntoElement, ParentElement, Render, ScrollHandle,
            StatefulInteractiveElement, Styled, Window,
        };

        struct Probe {
            scroll: ScrollHandle,
        }

        impl Render for Probe {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                div()
                    .id("scroll")
                    .debug_selector(|| "scroll".to_owned())
                    .size_full()
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .child(div().h(px(4000.)))
            }
        }

        let scroll = ScrollHandle::new();
        let tracked = scroll.clone();
        let (_view, cx) = cx.add_window_view(move |_, _| Probe { scroll: tracked });
        cx.run_until_parked();
        let start = cx
            .debug_bounds("scroll")
            .expect("the list must paint")
            .center();

        let mut guard = FlingGuard::new(GestureTuning::default());
        let mut next_id = 100;
        let mut relay = |event: TouchEvent, cx: &mut gpui::VisualTestContext| {
            guard.relay(
                event,
                || {
                    next_id += 1;
                    TouchId(next_id)
                },
                &mut |event| cx.simulate_event(event),
            );
        };

        relay(
            TouchEvent {
                id: TouchId(1),
                phase: TouchPhase::Started,
                position: start,
                ..Default::default()
            },
            cx,
        );
        for step in 1..=6 {
            relay(
                TouchEvent {
                    id: TouchId(1),
                    phase: TouchPhase::Moved,
                    position: point(start.x, start.y - px(40.0 * step as f32)),
                    ..Default::default()
                },
                cx,
            );
        }
        relay(
            TouchEvent {
                id: TouchId(1),
                phase: TouchPhase::Ended,
                position: point(start.x, start.y - px(240.0)),
                ..Default::default()
            },
            cx,
        );

        let released = scroll.offset().y;
        assert!(released < px(0.), "the swipe must scroll the list");

        std::thread::sleep(Duration::from_millis(50));
        for _ in 0..2 {
            cx.update(|window, cx| {
                window.simulate_next_frame(cx);
            });
            std::thread::sleep(Duration::from_millis(16));
        }

        assert!(
            scroll.offset().y < released,
            "a guarded release must keep coasting: released={released:?}, now={:?}",
            scroll.offset().y
        );
    }

    /// The guard's purpose, end to end: GPUI's recognizer never offers a
    /// contact that catches a fling as a touch drag, so a control that claims
    /// drags (a scrollbar thumb) cannot be grabbed while its content coasts.
    #[gpui::test]
    fn a_contact_that_lands_on_a_fling_is_still_offered_as_a_drag(cx: &mut gpui::TestAppContext) {
        use gpui::{
            canvas, div, Context, IntoElement, ParentElement, Render, Styled, TouchDragEvent,
            Window,
        };
        use std::{cell::Cell, rc::Rc};

        /// Claims drags that begin in a strip along the right edge, the way
        /// a scrollbar thumb does; the rest of the window pans.
        struct DragClaimer {
            claimed: Rc<Cell<usize>>,
        }

        const THUMB_LEFT: Pixels = px(90.);

        impl Render for DragClaimer {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                let claimed = self.claimed.clone();
                div().size_full().child(
                    canvas(
                        |_, _, _| (),
                        move |_, _, window, _| {
                            let claimed = claimed.clone();
                            window.on_mouse_event(
                                move |event: &TouchDragEvent, phase, window, cx| {
                                    if phase.bubble()
                                        && event.phase == TouchPhase::Started
                                        && event.start_position.x >= THUMB_LEFT
                                    {
                                        claimed.set(claimed.get() + 1);
                                        window.prevent_default();
                                        cx.stop_propagation();
                                    }
                                },
                            );
                        },
                    )
                    .size_full(),
                )
            }
        }

        for guarded in [false, true] {
            let claimed = Rc::new(Cell::new(0));
            let (_, cx) = cx.add_window_view({
                let claimed = claimed.clone();
                move |_, _| DragClaimer { claimed }
            });
            cx.update(|window, cx| window.draw(cx).clear(cx));
            let mut guard = FlingGuard::new(GestureTuning::default());
            let mut next_id = 100;
            let mut clock = Instant::now();
            let mut relay = |event: TouchEvent, cx: &mut gpui::VisualTestContext| {
                if guarded {
                    guard.relay_at(
                        event,
                        clock,
                        || {
                            next_id += 1;
                            TouchId(next_id)
                        },
                        &mut |event| cx.simulate_event(event),
                    );
                } else {
                    cx.simulate_event(event);
                }
                clock += ms(16);
            };

            // A fast upward swipe, released with velocity: the recognizer is
            // now coasting.
            relay(touch(1, TouchPhase::Started, 50., 400.), cx);
            for step in 1..=6 {
                relay(
                    touch(1, TouchPhase::Moved, 50., 400. - 40. * step as f32),
                    cx,
                );
            }
            relay(touch(1, TouchPhase::Ended, 50., 160.), cx);
            assert_eq!(claimed.get(), 0, "a swipe is a pan, not a drag");

            // A contact that lands while it coasts.
            relay(touch(2, TouchPhase::Started, 95., 20.), cx);
            relay(touch(2, TouchPhase::Ended, 95., 20.), cx);
            assert_eq!(
                claimed.get(),
                usize::from(guarded),
                "guarded={guarded}: only a guarded contact is offered as a drag"
            );
        }
    }
}
