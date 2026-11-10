use crate::shared::ui::views::design::theme;

use std::cell::Cell;
use std::rc::Rc;

use gpui::{
    App, Background, Bounds, Corners, CursorStyle, Element, ElementId, GlobalElementId,
    HitboxBehavior, Hsla, InspectorElementId, IntoElement, LayoutId, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, Pixels, Rgba, ScrollEvent, Style, Window, point, quad, relative,
    size,
};

type Change = Rc<dyn Fn(f32, bool, &mut Window, &mut App) + 'static>;

/// Every horizontal bar shares this thickness.
pub const HEIGHT: f32 = 8.0;

/// Every horizontal bar shares the same empty-track colour.
pub const TRACK: Rgba = crate::shared::ui::views::design::theme::NEUTRAL_800;

/// The band a finger can grab around the track. A silhouette the design keeps
/// hairline-thin is still aimed at with the whole pad of a finger.
const TOUCH: f32 = 44.0;

/// A rounded progress track with a circular thumb.
pub struct Bar {
    fraction: f32,
    width: Option<f32>,
    height: f32,
    filled: Rgba,
    empty: Rgba,
    thumb: f32,
    enabled: bool,
    dragging: bool,
    flat: bool,
    on_change: Change,
}

impl Bar {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        fraction: f32,
        width: Option<f32>,
        height: f32,
        filled: Rgba,
        empty: Rgba,
        thumb: f32,
        enabled: bool,
        dragging: bool,
        on_change: impl Fn(f32, bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            fraction: fraction.clamp(0.0, 1.0),
            width,
            height,
            filled,
            empty,
            thumb,
            enabled,
            dragging,
            flat: false,
            on_change: Rc::new(on_change),
        }
    }

    /// Drops the rounded ends so the bar reads as an edge-to-edge strip.
    pub fn flat(mut self) -> Self {
        self.flat = true;
        self
    }
}

fn dim(color: Rgba) -> Rgba {
    let mut hsla = Hsla::from(color);
    hsla.s *= 0.5;
    crate::shared::ui::views::design::theme::shade(Rgba::from(hsla), 0.75)
}

impl IntoElement for Bar {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for Bar {
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
        let width = match self.width {
            Some(width) => gpui::Length::Definite(theme::unit(width).into()),
            None => gpui::Length::Definite(relative(1.0)),
        };

        let height = gpui::Length::Definite(theme::unit(self.height).into());
        let layout = window.request_layout(
            Style {
                size: gpui::Size { width, height },
                flex_grow: if self.width.is_some() { 0.0 } else { 1.0 },
                flex_shrink: 1.0,
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
        // A finger needs a bigger target than the line it drags: the band is
        // touch-sized around the silent track, not drawn, so the bar keeps its
        // hairline look and a miss near the thumb still starts the seek.
        let reach = self.height.max(self.thumb * 2.0).max(TOUCH);
        let target = Bounds {
            origin: point(bounds.left(), bounds.center().y - theme::unit(reach * 0.5)),
            size: size(bounds.size.width, theme::unit(reach)),
        };

        window.insert_hitbox(target, HitboxBehavior::Normal)
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
        let top = (bounds.center().y - theme::unit(self.height * 0.5)).round();
        let track = Bounds {
            origin: point(bounds.left(), top),
            size: size(bounds.size.width, theme::unit(self.height)),
        };

        let radius = theme::unit(self.height * 0.5);
        let corners = if self.flat {
            Corners::default()
        } else {
            Corners::from(radius)
        };

        let (filled, empty) = if self.enabled {
            (self.filled, self.empty)
        } else {
            (dim(self.filled), dim(self.empty))
        };

        window.paint_quad(quad(
            track,
            corners,
            Background::from(empty),
            gpui::Edges::default(),
            Hsla::transparent_black(),
            gpui::BorderStyle::Solid,
        ));

        let fill_width = track.size.width.as_f32() * self.fraction;
        if fill_width > 0.0 {
            let fill = Bounds {
                origin: track.origin,
                size: size(theme::unit(fill_width), track.size.height),
            };

            window.paint_quad(quad(
                fill,
                corners,
                Background::from(filled),
                gpui::Edges::default(),
                Hsla::transparent_black(),
                gpui::BorderStyle::Solid,
            ));
        }

        let thumb_center = track.left().as_f32() + fill_width;
        let dragging = self.dragging;
        let radius = if dragging {
            self.thumb * 1.1
        } else {
            self.thumb
        };

        let thumb = Bounds {
            origin: point(
                theme::unit(thumb_center) - theme::unit(radius),
                track.center().y - theme::unit(radius),
            ),
            size: size(theme::unit(radius * 2.0), theme::unit(radius * 2.0)),
        };

        window.paint_quad(quad(
            thumb,
            Corners::from(theme::unit(radius)),
            Background::from(filled),
            gpui::Edges::default(),
            Hsla::transparent_black(),
            gpui::BorderStyle::Solid,
        ));

        let hitbox = hitbox.clone();
        let fraction = move |position: gpui::Point<Pixels>| {
            ((position.x.as_f32() - bounds.left().as_f32()) / bounds.size.width.as_f32())
                .clamp(0.0, 1.0)
        };

        if !self.enabled {
            return;
        }

        let pointer = window.mouse_position();
        let thumb_x = track.left() + theme::unit(fill_width);
        let cursor = if dragging {
            CursorStyle::ClosedHand
        } else if hitbox.contains(&pointer) && (pointer.x - thumb_x).abs() <= theme::unit(10.0) {
            CursorStyle::OpenHand
        } else {
            CursorStyle::PointingHand
        };

        window.set_cursor_style(cursor, &hitbox);

        // Pressing only previews; the release commits, so a tap seeks too.
        let armed = Rc::new(Cell::new(false));

        let change = self.on_change.clone();
        let down = hitbox.clone();
        let held = armed.clone();
        window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
            if phase.bubble()
                && event.button == MouseButton::Left
                && down.is_hovered(window)
                && down.contains(&event.position)
            {
                held.set(true);
                change(fraction(event.position), false, window, cx);
                cx.stop_propagation();
            }
        });

        let change = self.on_change.clone();
        let held = armed.clone();
        window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
            if phase.bubble() && (held.replace(false) || dragging) {
                change(fraction(event.position), true, window, cx);
            }
        });

        if self.dragging {
            let change = self.on_change.clone();
            window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
                if !phase.bubble() {
                    return;
                }

                let commit = event.pressed_button != Some(MouseButton::Left);
                change(fraction(event.position), commit, window, cx);
            });
        }

        // A finger never hovers: the recognizer offers a touch drag, and taking
        // it (prevent_default) keeps the pan from scrolling the page instead.
        // Every bar sees the same touch stream, so a move only counts when this
        // bar owns the drag; otherwise the volume slider would also seek.
        let change = self.on_change.clone();
        let area = hitbox.clone();
        let held = armed.clone();
        window.on_mouse_event(move |event: &gpui::TouchDragEvent, phase, window, cx| {
            if !phase.bubble() {
                return;
            }

            if event.phase == gpui::TouchPhase::Started {
                if !area.is_hovered_at(event.start_position, window) {
                    return;
                }

                held.set(true);
                window.prevent_default();
            } else if !dragging && !held.get() {
                return;
            }

            let commit = matches!(
                event.phase,
                gpui::TouchPhase::Ended | gpui::TouchPhase::Cancelled
            );
            if commit {
                held.set(false);
            }

            change(fraction(event.position), commit, window, cx);
        });

        let change = self.on_change.clone();
        let area = hitbox.clone();
        let current = Rc::new(Cell::new(self.fraction));
        window.on_mouse_event(move |event: &ScrollEvent, phase, window, cx| {
            if phase.bubble() && area.is_hovered(window) {
                let delta = event.delta.pixel_delta(window.line_height()).y.as_f32();
                let step = (delta * 0.002).clamp(-0.05, 0.05);
                if step.abs() > 0.0005 {
                    let next = (current.get() + step).clamp(0.0, 1.0);
                    current.set(next);
                    cx.stop_propagation();
                    change(next, true, window, cx);
                }
            }
        });
    }
}

#[cfg(test)]
#[path = "progress.test.rs"]
mod tests;
