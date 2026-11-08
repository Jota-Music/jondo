use std::rc::Rc;
use std::time::Instant;

use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, AppContext as _, Context, DragMoveEvent, ElementId, InteractiveElement,
    IntoElement, MouseButton, MouseDownEvent, MouseUpEvent, ParentElement,
    StatefulInteractiveElement, Styled, Window, div,
};
use gpui_kit::base::FocusTrapElement as _;

use crate::shared::ui::views::components::button;
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
            body: body.into_any_element(),
        }
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
            window.request_animation_frame();
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
        let close = self.close.clone();
        let scrim_close = close.clone();
        let grab_id = ElementId::Name(format!("{id}-grab").into());
        let id_press = id.clone();
        let id_move = id.clone();
        let id_up = id.clone();
        let id_out = id.clone();
        let id_measure = id.clone();
        let id_begin = id.clone();

        let release_up = close.clone();
        let release_out = close.clone();
        let dismiss = close.clone();
        let down = cx.listener(move |app, event: &MouseDownEvent, window, cx| {
            if mobile {
                press(&id_press, event.position.y.as_f32());
            } else {
                (dismiss)(app, window, cx);
            }

            cx.stop_propagation();
        });
        // The sheet drag is armed from the whole panel, but a control that owns
        // the gesture (a playhead, the disc, a reorderable row) stops the
        // mouse down so the sheet leaves the drag to it.
        let moved = cx.listener(move |_app, event: &DragMoveEvent<Grab>, _window, cx| {
            drag(&id_move, event.event.position.y.as_f32());
            cx.notify();
        });
        let up = cx.listener(move |app, _: &MouseUpEvent, window, cx| {
            if release_sheet(&id_up) {
                (release_up)(app, window, cx);
            }

            cx.notify();
        });
        let out = cx.listener(move |app, _: &MouseUpEvent, window, cx| {
            if release_sheet(&id_out) {
                (release_out)(app, window, cx);
            }

            cx.notify();
        });

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
            );

        let mut panel = div()
            .relative()
            .w_full()
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
                    .child(button::icon_button(
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
            );
        if !mobile {
            // Opaque to the wrapper, so a press on the panel is not mistaken for
            // a press outside it. Phones cannot do this: the drag is armed from
            // the wrapper, and occluding the panel would swallow the gesture.
            panel = panel.occlude().opacity(fade);
        }

        // Tab and Shift-Tab cycle inside the sheet instead of reaching the page
        // behind the scrim. The focus handle is the container the trap owns.
        let panel = panel
            .id(ElementId::Name(format!("{id}-panel").into()))
            .track_focus(&focus)
            .focus_trap(id.clone(), &focus);

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
                .bottom(theme::unit(clearance - top))
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

        wrapper = wrapper
            .on_mouse_down(MouseButton::Left, down)
            .when(mobile, move |wrapper| {
                wrapper
                    .on_drag(Grab, move |_, _, _, cx| {
                        begin(&id_begin);
                        cx.new(|_| Grab)
                    })
                    .on_drag_move::<Grab>(moved)
                    .on_mouse_up(MouseButton::Left, up)
                    .on_mouse_up_out(MouseButton::Left, out)
            });

        STATES.with(|states| states.borrow_mut().insert(id, state));

        div()
            .absolute()
            .inset_0()
            .occlude()
            .child(backdrop)
            .child(wrapper)
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
                top = from + (screen_height + BAR - from) * out(t);
                scrim = 0.7 * held * (1.0 - out(t));
                animating = t < 1.0;
            } else if let Some(motion) = &state.settle {
                let t = progress(motion.at, DURATION, now);
                top = motion.from * (1.0 - snap(t));
                scrim *= 1.0 - (top / CLOSE).min(1.0) * 0.85;
                animating = t < 1.0;
                if !animating {
                    state.settle = None;
                    state.offset = 0.0;
                }
            } else if state.active {
                top = state.offset;
                scrim *= 1.0 - (state.offset / CLOSE).min(1.0) * 0.85;
            } else if state.height <= 0.0 {
                top = screen_height + BAR;
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

        (top, scrim, fade, animating)
    }
}
