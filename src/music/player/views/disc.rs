use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;

use gpui::{
    App, Background, Bounds, BoxShadow, Context, Corners, CursorStyle, Div, Element, ElementId,
    GlobalElementId, HitboxBehavior, Hsla, InspectorElementId, IntoElement, LayoutId, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, PathBuilder, Pixels, RenderImage,
    Rgba, Style, Styled, TouchDragEvent, TouchPhase, Window, div, point, quad, relative, size,
};

use crate::shared::ui::views::components::button;
use crate::shared::ui::views::components::paint::{self, Scope};
use crate::shared::ui::views::design::icons;
use crate::shared::ui::views::design::theme;
use crate::shell::app::{Drag, Jondo};

pub type Seek = Rc<dyn Fn(f32, bool, &mut Window, &mut App) + 'static>;

thread_local! {
    static OVER_RING: Cell<bool> = const { Cell::new(false) };
}

/// The vinyl: a cover inside a dark disc with a seek ring around it.
pub struct Ring {
    pub cover: Option<Arc<RenderImage>>,
    pub progress: f32,
    pub accent: Rgba,
    pub enabled: bool,
    pub dragging: bool,
    pub on_seek: Seek,
}

impl Ring {
    pub fn new(
        cover: Option<Arc<RenderImage>>,
        progress: f32,
        accent: Rgba,
        enabled: bool,
        dragging: bool,
        on_seek: impl Fn(f32, bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            cover,
            progress,
            accent,
            enabled,
            dragging,
            on_seek: Rc::new(on_seek),
        }
    }
}

impl IntoElement for Ring {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for Ring {
    type RequestLayoutState = ();
    type PrepaintState = gpui::Hitbox;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _global_id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let layout = window.request_layout(
            Style {
                size: gpui::Size {
                    width: relative(1.0).into(),
                    height: relative(1.0).into(),
                },
                flex_shrink: 0.0,
                ..Default::default()
            },
            vec![],
            cx,
        );
        (layout, ())
    }

    fn prepaint(
        &mut self,
        _global_id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {
        window.insert_hitbox(bounds, HitboxBehavior::Normal)
    }

    fn paint(
        &mut self,
        _global_id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        hitbox: &mut Self::PrepaintState,
        window: &mut Window,
        _cx: &mut App,
    ) {
        let center = bounds.center();
        let radius = bounds.size.width.as_f32() * 0.5;
        let ring_radius = radius - 4.0;

        window.paint_quad(quad(
            bounds,
            Corners::from(theme::unit(radius)),
            Background::from(theme::ZINC_900),
            gpui::Edges::default(),
            Hsla::transparent_black(),
            gpui::BorderStyle::Solid,
        ));

        if let Some(cover) = self.cover.clone() {
            let inner = Bounds {
                origin: point(
                    center.x - theme::unit(radius - 1.0),
                    center.y - theme::unit(radius - 1.0),
                ),
                size: size(
                    theme::unit(radius * 2.0 - 2.0),
                    theme::unit(radius * 2.0 - 2.0),
                ),
            };

            window
                .paint_image(
                    inner,
                    inner,
                    Corners::from(theme::unit(radius - 1.0)),
                    cover,
                    0,
                    false,
                )
                .ok();
            window.paint_quad(quad(
                inner,
                Corners::from(theme::unit(radius - 1.0)),
                Background::from(theme::alpha(theme::ZINC_900, 0.6)),
                gpui::Edges::default(),
                Hsla::transparent_black(),
                gpui::BorderStyle::Solid,
            ));
        }

        let guide = theme::alpha(self.accent, 0.2);
        if let Some(path) = ring(center, ring_radius, 8.0, 0.0, std::f32::consts::TAU) {
            window.paint_path(path, Background::from(guide));
        }

        if self.enabled && self.progress > 0.0 {
            let start = -std::f32::consts::FRAC_PI_2;
            let end = start + self.progress.clamp(0.0, 1.0) * std::f32::consts::TAU;
            if let Some(path) = ring(center, ring_radius, 8.0, start, end) {
                window.paint_path(path, Background::from(self.accent));
            }

            for angle in [start, end] {
                let position = point(
                    center.x + theme::unit(angle.cos() * ring_radius),
                    center.y + theme::unit(angle.sin() * ring_radius),
                );
                window.paint_quad(quad(
                    Bounds {
                        origin: point(position.x - theme::unit(4.0), position.y - theme::unit(4.0)),
                        size: size(theme::unit(8.0), theme::unit(8.0)),
                    },
                    Corners::from(theme::unit(4.0)),
                    Background::from(self.accent),
                    gpui::Edges::default(),
                    Hsla::transparent_black(),
                    gpui::BorderStyle::Solid,
                ));
            }
        }

        let center_x = center.x.as_f32();
        let center_y = center.y.as_f32();
        let fraction = move |position: gpui::Point<Pixels>| {
            let dx = position.x.as_f32() - center_x;
            let dy = position.y.as_f32() - center_y;
            let theta = (dy as f64).atan2(dx as f64);
            ((theta + std::f64::consts::FRAC_PI_2).rem_euclid(std::f64::consts::TAU)
                / std::f64::consts::TAU) as f32
        };

        let on_ring = move |position: gpui::Point<Pixels>| {
            let dx = position.x.as_f32() - center_x;
            let dy = position.y.as_f32() - center_y;
            ((dx * dx + dy * dy).sqrt() - ring_radius).abs() <= 12.0
        };

        if self.dragging {
            window.set_cursor_style(CursorStyle::ClosedHand, hitbox);
        } else if self.enabled && OVER_RING.with(Cell::get) {
            window.set_cursor_style(CursorStyle::OpenHand, hitbox);
        }

        if self.enabled {
            let area = hitbox.clone();
            window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
                if !phase.bubble() {
                    return;
                }
                let over = area.is_hovered(window) && on_ring(event.position);
                if OVER_RING.with(|state| state.replace(over)) != over {
                    paint::paint(Scope::Shell, cx);
                }
            });
            let seek = self.on_seek.clone();
            let ring = hitbox.clone();
            window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
                if phase.bubble()
                    && event.button == MouseButton::Left
                    && ring.is_hovered(window)
                    && on_ring(event.position)
                {
                    seek(fraction(event.position), false, window, cx);
                    cx.stop_propagation();
                }
            });

            if self.dragging {
                let seek = self.on_seek.clone();
                window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
                    if phase.bubble() && event.pressed_button == Some(MouseButton::Left) {
                        seek(fraction(event.position), false, window, cx);
                    }
                });
                let seek = self.on_seek.clone();
                window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
                    if phase.bubble() {
                        seek(fraction(event.position), true, window, cx);
                    }
                });
            }

            // A finger has no pointer capture: the recognizer offers a touch
            // drag instead. As in Jota's circular progress, the press arms on
            // the ring, the moves retarget by angle from anywhere, and the
            // release commits. Taking the gesture (prevent_default) is what
            // keeps the sheet from scrolling under the finger instead.
            let dragging = self.dragging;
            let seek = self.on_seek.clone();
            let area = hitbox.clone();
            let held = Rc::new(Cell::new(false));
            window.on_mouse_event(move |event: &TouchDragEvent, phase, window, cx| {
                if !phase.bubble() {
                    return;
                }

                if event.phase == TouchPhase::Started {
                    // A sheet over the disc covers it without moving it, so
                    // the ring must be asked whether it is still the surface
                    // under the finger, not just whether it contains it.
                    if !area.is_hovered_at(event.start_position, window)
                        || !on_ring(event.start_position)
                    {
                        return;
                    }

                    held.set(true);
                    window.prevent_default();
                } else if !dragging && !held.get() {
                    return;
                }

                let commit = matches!(event.phase, TouchPhase::Ended | TouchPhase::Cancelled);
                if commit {
                    held.set(false);
                }

                seek(fraction(event.position), commit, window, cx);
            });
        }
    }
}

fn ring(
    center: gpui::Point<Pixels>,
    radius: f32,
    width: f32,
    start: f32,
    end: f32,
) -> Option<gpui::Path<Pixels>> {
    let outer = radius + width * 0.5;
    let inner = radius - width * 0.5;
    let steps = (((end - start).abs() / std::f32::consts::TAU) * 180.0)
        .ceil()
        .max(2.0) as usize;
    let mut builder = PathBuilder::fill();
    for index in 0..=steps {
        let angle = start + (end - start) * index as f32 / steps as f32;
        let point = point(
            center.x + theme::unit(angle.cos() * outer),
            center.y + theme::unit(angle.sin() * outer),
        );
        if index == 0 {
            builder.move_to(point);
        } else {
            builder.line_to(point);
        }
    }

    for index in (0..=steps).rev() {
        let angle = start + (end - start) * index as f32 / steps as f32;
        builder.line_to(point(
            center.x + theme::unit(angle.cos() * inner),
            center.y + theme::unit(angle.sin() * inner),
        ));
    }

    builder.close();
    builder.build().ok()
}

fn depth() -> Vec<BoxShadow> {
    vec![
        BoxShadow::new(theme::unit(0.0), theme::unit(4.0), theme::BINARY.into())
            .blur_radius(theme::unit(4.0)),
    ]
}

pub(super) fn disc(app: &mut Jondo, cx: &mut Context<Jondo>, size: f32, enabled: bool) -> Div {
    let duration = app.player.read(cx).duration;
    let shown = app
        .player
        .read(cx)
        .seek_preview
        .unwrap_or(app.player.read(cx).progress);
    let fraction = if duration > 0.0 {
        (shown / duration).clamp(0.0, 1.0) as f32
    } else {
        0.0
    };

    let accent = app.accent;
    let url = app
        .player
        .read(cx)
        .current()
        .and_then(|song| song.cover.clone());
    let angle = app.disc_spin;
    let spin = url.as_deref().and_then(|url| app.covers.spin(url, angle));
    let cover = spin.or_else(|| {
        url.as_deref()
            .and_then(|url| app.covers.image(url, size * 2.0))
    });
    let dragging = app.ui.drag == Some(Drag::Disc);
    let playing = app.player.read(cx).playing;
    let loading = app.player.read(cx).loading;
    let binary = app.binary;

    div()
        .relative()
        .w(theme::unit(size))
        .h(theme::unit(size))
        .flex_none()
        .rounded_full()
        .shadow(depth())
        .child(Ring::new(cover, fraction, accent, enabled, dragging, {
            let entity = cx.entity();
            move |value, commit, _, cx| {
                entity.update(cx, |app, cx| {
                    if commit {
                        app.ui.drag = None;
                        let at = value as f64 * app.player.read(cx).duration;
                        app.player
                            .update(cx, |player, _| player.seek_preview = None);
                        app.seek(at, cx);
                    } else {
                        app.ui.drag = Some(Drag::Disc);
                        let duration = app.player.read(cx).duration;
                        app.player.update(cx, |player, _| {
                            player.seek_preview = Some(value as f64 * duration);
                        });
                    }

                    app.notify_shell(cx);
                });
            }
        }))
        .child(
            div()
                .absolute()
                .left(theme::unit(size * 0.5 - 64.0 - 16.0))
                .top(theme::unit(size * 0.5 - 16.0))
                .w(theme::unit(32.0))
                .h(theme::unit(32.0))
                .child(
                    div()
                        .absolute()
                        .left(theme::unit(4.0))
                        .top(theme::unit(8.0))
                        .child(icons::icon(
                            icons::SKIP_BACK,
                            24.0,
                            theme::alpha(theme::BINARY, 0.6),
                        )),
                )
                .child(button::icon_flat_in(
                    Scope::Shell,
                    "disc-prev",
                    icons::SKIP_BACK,
                    24.0,
                    if enabled { accent } else { theme::ZINC_700 },
                    32.0,
                    enabled,
                    cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                        app.previous(cx);
                        cx.notify();
                    }),
                )),
        )
        .child(
            div()
                .absolute()
                .left(theme::unit(size * 0.5 + 64.0 - 16.0))
                .top(theme::unit(size * 0.5 - 16.0))
                .w(theme::unit(32.0))
                .h(theme::unit(32.0))
                .child(
                    div()
                        .absolute()
                        .left(theme::unit(4.0))
                        .top(theme::unit(8.0))
                        .child(icons::icon(
                            icons::SKIP_FORWARD,
                            24.0,
                            theme::alpha(theme::BINARY, 0.6),
                        )),
                )
                .child(button::icon_flat_in(
                    Scope::Shell,
                    "disc-next",
                    icons::SKIP_FORWARD,
                    24.0,
                    if enabled { accent } else { theme::ZINC_700 },
                    32.0,
                    enabled,
                    cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                        app.next(cx);
                        cx.notify();
                    }),
                )),
        )
        .child(
            div()
                .absolute()
                .left(theme::unit(size * 0.5 - 24.0))
                .top(theme::unit(size * 0.5 - 24.0))
                .w(theme::unit(48.0))
                .h(theme::unit(48.0))
                .rounded_full()
                .shadow(depth())
                .child(if loading {
                    button::circle_spinner(48.0, 24.0, accent, binary).into_any_element()
                } else {
                    button::circle_in(
                        Scope::Shell,
                        "disc-play",
                        if playing { icons::PAUSE } else { icons::PLAY },
                        48.0,
                        24.0,
                        if enabled { accent } else { theme::ZINC_700 },
                        binary,
                        enabled,
                        cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                            app.toggle(cx);
                            cx.notify();
                        }),
                    )
                    .into_any_element()
                }),
        )
}

#[cfg(test)]
#[path = "disc.test.rs"]
mod tests;
