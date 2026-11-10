use std::rc::Rc;
use std::time::Instant;

use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, AppContext as _, Context, CursorStyle, DragMoveEvent, ElementId,
    InteractiveElement, IntoElement, MouseButton, MouseDownEvent, MouseUpEvent, ParentElement,
    ScrollEvent, ScrollSource, StatefulInteractiveElement, Styled, TouchDragEvent, TouchPhase,
    Window, canvas, div,
};
use gpui_kit::base::FocusTrapElement as _;

use crate::shared::ui::views::components::button;
use crate::shared::ui::views::components::paint::{self, Scope};
use crate::shared::ui::views::design::icons;
use crate::shared::ui::views::design::theme;
use crate::shell::app::Jondo;

/// How long the sheet takes to slide in, and how long the caller keeps it
/// mounted after it starts closing.
use super::*;
use super::{drag::*, state::*};

/// A bottom sheet on phones and a centered dialog above 640px. On the sheet,
/// drag it down to dismiss: it follows the pointer, the scrim fades with it,
/// and a release settles back with the same curve a native drawer uses.
pub struct Sheet {
    id: ElementId,
    closing: Option<Instant>,
    mobile: Option<bool>,
    above_player: bool,
    bar: bool,
    width: f32,
    max_height: Option<f32>,
    close: Close,
    at_top: Option<Rc<dyn Fn() -> bool>>,
    body: AnyElement,
}

impl Sheet {
    pub fn new(
        id: impl Into<ElementId>,
        closing: Option<Instant>,
        close: impl Fn(&mut Jondo, &mut Window, &mut Context<Jondo>) + 'static,
        body: impl IntoElement,
    ) -> Self {
        Self {
            id: id.into(),
            closing,
            mobile: None,
            above_player: true,
            bar: false,
            width: theme::CONTENT_WIDTH,
            max_height: None,
            close: Rc::new(close),
            at_top: None,
            body: body.into_any_element(),
        }
    }

    /// Tells the sheet when the scrollable body it covers is still at the top,
    /// so a downward pan on the body pulls the sheet and an upward one scrolls
    /// it. Without this the body drags the sheet at every offset.
    pub fn at_top(mut self, at_top: impl Fn() -> bool + 'static) -> Self {
        self.at_top = Some(Rc::new(at_top));
        self
    }

    pub fn mobile(mut self, mobile: bool) -> Self {
        self.mobile = Some(mobile);
        self
    }

    pub fn above_player(mut self, above: bool) -> Self {
        self.above_player = above;
        self
    }

    pub fn bar(mut self, bar: bool) -> Self {
        self.bar = bar;
        self
    }

    pub fn max_height(mut self, height: f32) -> Self {
        self.max_height = Some(height);
        self
    }

    pub fn show(self, window: &mut Window, cx: &mut Context<Jondo>) -> AnyElement {
        let viewport = window.viewport_size();
        let screen_width = viewport.width.as_f32();
        let screen_height = viewport.height.as_f32();
        let mobile = self.mobile.unwrap_or(screen_width < 640.0);
        // The overlay container already ends on the phone navigation, so a
        // sheet only lifts by the mini player bar when one is on screen.
        let clearance = if mobile && self.bar && self.above_player {
            BAR
        } else {
            0.0
        };

        let max_height = self.max_height.unwrap_or_else(|| {
            if mobile {
                (screen_height * 0.7)
                    .min(screen_height - clearance - 16.0)
                    .max(160.0)
            } else {
                screen_height * 0.85
            }
        });
        let width = (screen_width - 32.0).min(self.width);

        let now = Instant::now();
        let id = self.id.clone();
        let mut state = STATES.with(|states| states.borrow_mut().remove(&id).unwrap_or_default());
        if self.closing.is_none() && state.closing_seen {
            state = State::default();
        }

        if self.closing.is_some() {
            state.closing_seen = true;
        }

        let (top, scrim, fade, animating) = self.motion(&mut state, mobile, screen_height, now);
        if animating {
            // A moving sheet is the only thing in the window that changes this
            // frame, so the frame is asked for as motion: the page behind the
            // scrim keeps the paint it already has instead of being rebuilt for
            // a slide it does not take part in.
            crate::shell::app::motion::ask(window);
        }

        let focus = window
            .use_keyed_state(
                ElementId::Name(format!("{id}-focus").into()),
                cx,
                |_, cx| cx.focus_handle(),
            )
            .read(cx)
            .clone();
        if !state.focused {
            state.focused = true;
            focus.focus(window, cx);
        }

        let pulled = state.active && state.offset > 0.0;
        // A mouse says what the drag can do: an open hand over the sheet it can
        // pull, a closed one while it is being pulled.
        let dragging = state.active;
        let cursor = if dragging {
            CursorStyle::ClosedHand
        } else {
            CursorStyle::OpenHand
        };
        let close = self.close.clone();
        let scrim_close = close.clone();
        let grab_id = ElementId::Name(format!("{id}-grab").into());
        let id_move = id.clone();
        let id_measure = id.clone();
        let id_begin = id.clone();

        let dismiss = close.clone();
        let down = cx.listener(move |app, _: &MouseDownEvent, window, cx| {
            (dismiss)(app, window, cx);
            cx.stop_propagation();
        });
        // The sheet drag is armed from the whole panel, but a control that owns
        // the gesture (a playhead, the disc, a reorderable row) stops the
        // mouse down so the sheet leaves the drag to it. The press is read on
        // the panel and not on the wrapper that centres it: the panel occludes
        // the window, so a press on a dialog never reaches the wrapper's
        // hitbox and a listener there could not arm.
        let press_id = id.clone();
        let press_panel = cx.listener(move |_app, event: &MouseDownEvent, _window, _cx| {
            press(&press_id, event.position.y.as_f32());
        });
        let moved = cx.listener(move |_app, event: &DragMoveEvent<Grab>, _window, cx| {
            drag(&id_move, event.event.position.y.as_f32());
            crate::shell::app::motion::note();
            cx.notify();
        });

        // The release is listened for on the window and not on the panel: a
        // fast drag can end with the pointer off the dialog, and a listener on
        // an element only hears the mouse up that lands on it.
        let release_entity = cx.entity();
        let release_close = self.close.clone();
        let release_id = id.clone();
        let mouse_up = canvas(
            |_, _, _| (),
            move |_, (), window, _cx| {
                let id = release_id.clone();
                let close = release_close.clone();
                let entity = release_entity.clone();
                window.on_mouse_event(move |_: &MouseUpEvent, phase, window, cx| {
                    if !phase.bubble() || !active(&id) {
                        return;
                    }

                    if release_sheet(&id) {
                        entity.update(cx, |app, cx| (close)(app, window, cx));
                    }

                    paint::paint(Scope::Shell, cx);
                });
            },
        )
        .absolute()
        .inset_0();

        // A finger has no mouse drag: the pointer recognizer offers a touch
        // drag instead. Claiming it from the strip above the body, and only
        // there, keeps a pan inside the sheet scrolling its own content.
        let entity = cx.entity();
        let touch_close = self.close.clone();
        let touch_id = id.clone();
        let touch = canvas(
            |_, _, _| (),
            move |area, (), window, _cx| {
                let id = touch_id.clone();
                let close = touch_close.clone();
                let entity = entity.clone();
                window.on_mouse_event(move |event: &TouchDragEvent, phase, window, cx| {
                    if !phase.bubble() {
                        return;
                    }

                    if event.phase == TouchPhase::Started {
                        if !area.contains(&event.start_position) {
                            return;
                        }

                        press(&id, event.start_position.y.as_f32());
                        begin(&id);
                        window.prevent_default();
                    } else if active(&id) {
                        if event.phase == TouchPhase::Moved {
                            drag(&id, event.position.y.as_f32());
                        } else if matches!(event.phase, TouchPhase::Ended | TouchPhase::Cancelled)
                            && release_sheet(&id)
                        {
                            entity.update(cx, |app, cx| (close)(app, window, cx));
                        }
                    } else {
                        return;
                    }

                    paint::paint(Scope::Shell, cx);
                });
            },
        )
        .absolute()
        .left(theme::unit(0.0))
        .top(theme::unit(0.0))
        .right(theme::unit(CLOSE_SIZE + CLOSE_TOP * 2.0))
        .bottom(theme::unit(0.0));

        // A finger pan never becomes a mouse drag, and claiming every touch on
        // the panel would kill the taps and the pan that scrolls the body. So
        // the panel listens to the recognized pan instead: a downward step from
        // the top of the body pulls the sheet, and every other step is left to
        // the body's own scroll. This is the whole-panel grab the mouse path
        // already has, with scrolling kept intact.
        let drag_entity = cx.entity();
        let drag_close = self.close.clone();
        let drag_at_top = self.at_top.clone();
        let drag_id = id.clone();
        let body_drag = canvas(
            |_, _, _| (),
            move |area, (), window, _cx| {
                let id = drag_id.clone();
                let close = drag_close.clone();
                let entity = drag_entity.clone();
                let at_top = drag_at_top.clone();
                window.on_mouse_event(move |event: &ScrollEvent, phase, window, cx| {
                    if !phase.bubble() || event.source != ScrollSource::Touch {
                        return;
                    }

                    // The scrim covers the page, so a touch pan while a sheet
                    // is up belongs to this layer: it either scrolls the modal
                    // body or pulls the sheet. Saying so keeps the page out of
                    // the frame, which is what made scrolling inside a modal
                    // rebuild the route behind it on every sample.
                    crate::shell::app::motion::note();

                    let step = event.delta.pixel_delta(theme::unit(20.0)).y.as_f32();
                    if event.touch_phase == TouchPhase::Started {
                        absorb(&id, false);
                        if step <= 0.0
                            || !area.contains(&event.position)
                            || at_top.as_ref().is_some_and(|at_top| !at_top())
                        {
                            return;
                        }

                        press(&id, event.position.y.as_f32());
                        begin(&id);
                        shift(&id, step);
                    } else if active(&id) {
                        match event.touch_phase {
                            TouchPhase::Moved => shift(&id, step),
                            TouchPhase::Ended => {
                                shift(&id, step);
                                absorb(&id, true);
                                if release_sheet(&id) {
                                    entity.update(cx, |app, cx| (close)(app, window, cx));
                                }
                            }
                            TouchPhase::Cancelled => {
                                abort(&id);
                                absorb(&id, true);
                            }
                            TouchPhase::Started => {}
                        }
                    } else if absorbing(&id) {
                        // The fling the recognizer keeps after the release
                        // belongs to the sheet: stop it from coasting the body
                        // the drag started on.
                        if matches!(event.touch_phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                            absorb(&id, false);
                        }
                    } else {
                        return;
                    }

                    cx.stop_propagation();
                    paint::paint(Scope::Shell, cx);
                });
            },
        )
        .absolute()
        .inset_0();

        let grip = div()
            .w(theme::unit(if pulled { 60.0 } else { 48.0 }))
            .h(theme::unit(6.0))
            .rounded_full()
            .bg(if pulled {
                theme::MUTED
            } else {
                theme::ZINC_600
            });

        // The handle strip is as tall as the close button, so the body can
        // start below it and nothing interactive ends up under the X.
        let handle = div()
            .relative()
            .w_full()
            .flex_none()
            .h(theme::unit(CLOSE_TOP + CLOSE_SIZE))
            .child(
                div()
                    .absolute()
                    .top(theme::unit(CLOSE_TOP))
                    .left(theme::unit(0.0))
                    .right(theme::unit(0.0))
                    .flex()
                    .justify_center()
                    .child(grip),
            )
            .child(touch);

        let mut panel = div()
            .relative()
            .w_full()
            .cursor(cursor)
            .when(!mobile, move |this| this.top(gpui::px(top)))
            .when(mobile, |this| {
                this.max_w(theme::unit(self.width.min(theme::CONTENT_WIDTH)))
            })
            .when(!mobile, |this| this.w(theme::unit(width)))
            .max_h(theme::unit(max_height))
            .flex()
            .flex_col()
            .overflow_hidden()
            .when(mobile, |this| this.rounded_t(theme::unit(16.0)))
            .when(!mobile, |this| this.rounded(theme::unit(16.0)))
            .bg(theme::SURFACE)
            .border_1()
            .border_color(theme::BORDER)
            .when(mobile, |this| this.border_b_0())
            .when(!cfg!(target_os = "android"), |this| this.shadow_2xl())
            .when(mobile, |this| this.child(handle))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h(theme::unit(0.0))
                    .child(self.body),
            )
            .child(
                div()
                    .absolute()
                    .top(theme::unit(CLOSE_TOP))
                    .right(theme::unit(CLOSE_TOP))
                    .child(button::icon_button_in(
                        Scope::Shell,
                        format!("{id}-close"),
                        icons::X,
                        20.0,
                        theme::MUTED,
                        CLOSE_SIZE,
                        true,
                        cx.listener(move |app, _: &gpui::ClickEvent, window, cx| {
                            (close)(app, window, cx);
                        }),
                    )),
            )
            // Painted last so its scroll listener runs first in the bubble
            // phase and can take the pan before the body scrolls.
            .when(mobile, |this| this.child(body_drag));
        // Opaque to the wrapper, so a press on the panel is not mistaken for a
        // press outside it, on phones too: a finger tap on the body lands on
        // the panel and must not reach the scrim behind it. The touch drag is
        // claimed by the handle strip and the pan canvas, not by the panel.
        panel = panel.occlude().opacity(fade);

        // Tab and Shift-Tab cycle inside the sheet instead of reaching the page
        // behind the scrim. The focus handle is the container the trap owns.
        let panel = panel
            .id(ElementId::Name(format!("{id}-panel").into()))
            .track_focus(&focus)
            .focus_trap(id.clone(), &focus)
            // A dialog on a wide window is dragged by its own surface: a press
            // on it never reaches the wrapper, which only centres it, because
            // the panel occludes the window.
            .when(!mobile, move |this| {
                this.on_mouse_down(MouseButton::Left, press_panel)
                    .on_drag(Grab, move |_, _, _, cx| {
                        begin(&id_begin);
                        cx.new(|_| Grab)
                    })
                    .on_drag_move::<Grab>(moved)
                    .child(mouse_up)
            });

        let backdrop = div()
            .absolute()
            .inset_0()
            .when(mobile, |this| this.bottom(theme::unit(clearance)))
            .bg(theme::black(scrim))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |app, _: &MouseDownEvent, window, cx| {
                    (scrim_close)(app, window, cx);
                }),
            );

        let mut wrapper = if mobile {
            div()
                .absolute()
                .left(theme::unit(0.0))
                .right(theme::unit(0.0))
                .bottom(theme::unit(clearance) - gpui::px(top))
                .flex()
                .justify_center()
                .on_children_prepainted(move |bounds, _, _| {
                    if let Some(bounds) = bounds.first() {
                        measure(&id_measure, bounds.size.height.as_f32());
                    }
                })
                .child(panel)
                .id(grab_id.clone())
        } else {
            div()
                .absolute()
                .inset_0()
                .flex()
                .items_center()
                .justify_center()
                .on_children_prepainted(move |bounds, _, _| {
                    if let Some(bounds) = bounds.first() {
                        measure(&id_measure, bounds.size.height.as_f32());
                    }
                })
                .child(panel)
                .id(grab_id.clone())
        };

        // The drag is a mouse gesture only: a finger drags from the handle strip
        // or from a pan at the top of the body, and claiming the press from the
        // panel would swallow every tap on a row. Above 640px the sheet is a
        // dialog and any press outside it dismisses it; the drag on the dialog
        // itself is armed from the panel.
        wrapper = wrapper.when(!mobile, move |wrapper| {
            wrapper.on_mouse_down(MouseButton::Left, down)
        });

        STATES.with(|states| states.borrow_mut().insert(id, state));

        // While a pull is in flight the hand stays closed wherever the pointer
        // goes. A per-element cursor cannot promise that: an element only sets
        // it while it is under the pointer, and a fast pull can outrun the
        // panel for a frame. This is the window-level equivalent of the web
        // setting the cursor on `body`.
        let cursor_layer = canvas(
            |_, _, _| (),
            move |_, (), window, _cx| {
                if dragging {
                    window.set_window_cursor_style(CursorStyle::ClosedHand);
                }
            },
        )
        .absolute()
        .inset_0();

        div()
            .absolute()
            .inset_0()
            .occlude()
            .child(backdrop)
            .child(wrapper)
            .child(cursor_layer)
            .into_any_element()
    }

    fn motion(
        &self,
        state: &mut State,
        mobile: bool,
        screen_height: f32,
        now: Instant,
    ) -> (f32, f32, f32, bool) {
        let mut top = 0.0;
        let mut animating = false;
        let mut scrim = 0.7;
        let mut fade = 1.0;

        if mobile {
            if let Some(at) = self.closing {
                if state.dismissed.is_none() {
                    state.dismissed = Some(visual(state, now));
                }

                let from = state.dismissed.unwrap_or(0.0);
                let t = progress(at, EXIT, now);
                let held = 1.0 - (from / CLOSE).min(1.0) * 0.85;
                let bar = theme::unit(BAR).as_f32();
                top = from + (screen_height + bar - from) * out(t);
                scrim = 0.7 * held * (1.0 - out(t));
                animating = t < 1.0;
            } else if let Some(motion) = &state.settle {
                let t = progress(motion.at, motion.duration, now);
                top = motion.from * (1.0 - snap(t));
                scrim *= 1.0 - (top / CLOSE).min(1.0) * 0.85;
                animating = t < 1.0;
                if !animating {
                    state.settle = None;
                }
            } else if state.active {
                top = state.offset;
                scrim *= 1.0 - (state.offset / CLOSE).min(1.0) * 0.85;
                animating = true;
            } else if state.height <= 0.0 {
                top = screen_height + theme::unit(BAR).as_f32();
                scrim = 0.0;
                animating = true;
            } else {
                if state.opened.is_none() {
                    state.opened = Some(now);
                }

                let t = progress(state.opened.unwrap(), DURATION, now);
                top = state.height * (1.0 - out(t));
                scrim *= out(t);
                animating = t < 1.0;
            }
        } else {
            if state.opened.is_none() {
                state.opened = Some(now);
                animating = true;
            }

            // A dialog is dragged down its own surface, so it follows the
            // pointer the same way the sheet does on a phone. It rests and
            // closes with a fade, so a dismissed dialog fades out where the
            // hand left it instead of jumping back to the centre first.
            if let Some(from) = state.dismissed {
                top = from;
            }

            if state.active {
                top = state.offset;
                animating = true;
            } else if let Some(motion) = &state.settle {
                let t = progress(motion.at, motion.duration, now);
                top = motion.from * (1.0 - snap(t));
                animating = t < 1.0;
                if !animating {
                    state.settle = None;
                }
            }

            let at = self.closing.or(state.opened);
            fade = match at {
                Some(at) => {
                    let t = progress(at, DURATION, now);
                    animating |= t < 1.0;
                    if self.closing.is_some() {
                        1.0 - out(t)
                    } else {
                        out(t)
                    }
                }
                None => 1.0,
            };

            scrim *= fade;
        }

        // The caller keeps a closing sheet mounted for `DURATION`, which is
        // longer than the exit itself: the frame that drops it has to be drawn
        // too, so the frames keep coming until the caller takes it down.
        animating |= self.closing.is_some();

        (top, scrim, fade, animating)
    }
}
