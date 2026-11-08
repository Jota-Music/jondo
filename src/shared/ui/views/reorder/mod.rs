use crate::shared::ui::views::design::theme;

use std::rc::Rc;

use gpui::{
    App, Bounds, Element, ElementId, GlobalElementId, Hitbox, HitboxBehavior, InspectorElementId,
    IntoElement, LayoutId, LongPressEvent, Pixels, Position, Style, TouchPhase, Window,
};

type Press = Rc<dyn Fn(LongPressEvent, &mut Window, &mut App) -> bool>;

/// The track rows in the queue and in a playlist share one height, so the
/// geometry below can name a row without asking the list.
pub const ROW: f32 = 64.0;
/// How far the pointer travels before a press becomes a drag.
pub const DRAG: f64 = 4.0;
/// The scrollbar column a touch drag leaves alone.
const GUTTER: f32 = 15.0;
/// The band at each end of the viewport where a drag scrolls the list.
const EDGE: f32 = 48.0;
/// Pixels per frame at the deepest point of the band.
const STEP: f32 = 16.0;

/// How far to scroll the list this frame for a pointer at `y`, in pixels, with
/// positive scrolling down. None when the pointer rests outside both edge
/// bands or the list has no room left in that direction.
pub fn delta(y: Pixels, viewport: Bounds<Pixels>, offset: Pixels, max: Pixels) -> Option<f32> {
    if max <= theme::unit(0.0) {
        return None;
    }

    let y = y.as_f32();
    let top = viewport.top().as_f32();
    let bottom = viewport.bottom().as_f32();
    let mut step = 0.0;
    if y < top + EDGE {
        step = -(((top + EDGE - y) / EDGE) * STEP).ceil();
    } else if y > bottom - EDGE {
        step = (((y - (bottom - EDGE)) / EDGE) * STEP).ceil();
    }

    step = step.clamp(-STEP, STEP);
    if step < 0.0 && offset >= theme::unit(0.0) {
        return None;
    }

    if step > 0.0 && -offset >= max {
        return None;
    }

    (step != 0.0).then_some(step)
}

/// The row under a pointer, read from the list geometry rather than from the
/// row that last saw the pointer, so a list scrolling under a resting pointer
/// still retargets.
pub fn index(y: Pixels, viewport: Bounds<Pixels>, offset: Pixels, row: f32, len: usize) -> usize {
    if len == 0 {
        return 0;
    }

    let relative = (y.as_f32() - viewport.top().as_f32()) - offset.as_f32();
    ((relative / row).floor() as isize).clamp(0, len as isize - 1) as usize
}

/// The long-press surface a touch drag starts from. It covers the list but
/// paints behind its rows, so a row's own long-press (a tooltip on a button)
/// claims the gesture first. A finger has no hover: the long press is the
/// hold that tells a reorder apart from the pan that scrolls the list.
pub struct Touch {
    handler: Press,
}

impl Touch {
    pub fn new(handler: impl Fn(LongPressEvent, &mut Window, &mut App) -> bool + 'static) -> Self {
        Self {
            handler: Rc::new(handler),
        }
    }
}

impl IntoElement for Touch {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for Touch {
    type RequestLayoutState = ();
    type PrepaintState = Hitbox;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style {
            position: Position::Absolute,
            ..Default::default()
        };
        style.inset.top = theme::unit(0.0).into();
        style.inset.left = theme::unit(0.0).into();
        style.inset.bottom = theme::unit(0.0).into();
        style.inset.right = theme::unit(GUTTER).into();
        (window.request_layout(style, None, cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        window: &mut Window,
        _: &mut App,
    ) -> Self::PrepaintState {
        window.insert_hitbox(bounds, HitboxBehavior::Normal)
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        hitbox: &mut Self::PrepaintState,
        window: &mut Window,
        _: &mut App,
    ) {
        let handler = self.handler.clone();
        let area = hitbox.clone();
        window.on_mouse_event(move |event: &LongPressEvent, phase, window, cx| {
            if !phase.bubble() {
                return;
            }

            if event.phase == TouchPhase::Started
                && (window.default_prevented() || !area.is_hovered_at(event.start_position, window))
            {
                return;
            }

            let claimed = handler(event.clone(), window, cx);
            if claimed && event.phase == TouchPhase::Started {
                window.prevent_default();
            }
        });
    }
}

#[cfg(test)]
#[path = "reorder.test.rs"]
mod tests;
