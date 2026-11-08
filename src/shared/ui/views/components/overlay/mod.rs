use crate::shared::ui::views::design::theme;

use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui::{Animation, AnimationExt, AnyElement, ElementId, IntoElement, Styled, ease_out_quint};

/// How long an overlay takes to appear or disappear.
pub const DURATION: Duration = Duration::from_millis(300);

/// A floating panel that fades while it settles into place. `lift` is how far
/// below its resting position it starts, in pixels.
pub fn panel<E: IntoElement + Styled + 'static>(
    id: impl Into<ElementId>,
    closing: Option<Instant>,
    lift: f32,
    element: E,
) -> AnyElement {
    animate(
        (id.into(), "panel").into(),
        closing,
        element,
        move |panel, delta| panel.opacity(delta).mt(theme::unit((1.0 - delta) * lift)),
    )
}

fn animate<E: IntoElement + Styled + 'static>(
    id: ElementId,
    closing: Option<Instant>,
    element: E,
    animate: impl Fn(E, f32) -> E + 'static,
) -> AnyElement {
    let animate = Rc::new(animate);
    let closing = closing.is_some();
    element
        .with_animation(
            (id, if closing { "out" } else { "in" }),
            Animation::new(DURATION).with_easing(ease_out_quint()),
            move |element, delta| animate.as_ref()(element, progress(delta, closing)),
        )
        .into_any_element()
}

fn progress(delta: f32, closing: bool) -> f32 {
    if closing { 1.0 - delta } else { delta }
}

#[cfg(test)]
#[path = "overlay.test.rs"]
mod tests;
