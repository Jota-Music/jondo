use std::rc::Rc;

use gpui::{
    App, HitboxBehavior, IntoElement, LongPressEvent, ParentElement, Pixels, Point, Styled,
    TouchPhase, Window, canvas, div,
};

/// Opens a context-sensitive action on a long press.
///
/// A finger has no right button, so a long press is the phone's equivalent of
/// a right click. The overlay sits over the trigger without blocking it: taps
/// and drags still reach the row underneath, and only the long press the
/// pointer recognizer offers is claimed. Its hitbox takes no mouse behaviour,
/// it only lets the press ask whether the trigger is still the surface under
/// the finger instead of a sheet that was opened over it.
pub fn hold(on_hold: impl Fn(Point<Pixels>, &mut Window, &mut App) + 'static) -> impl IntoElement {
    let on_hold = Rc::new(on_hold);
    div().absolute().inset_0().child(
        canvas(
            |bounds, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal),
            move |_, hitbox, window, _| {
                window.on_mouse_event(move |event: &LongPressEvent, phase, window, cx| {
                    if !phase.bubble()
                        || event.phase != TouchPhase::Started
                        || window.default_prevented()
                        || !hitbox.is_hovered_at(event.start_position, window)
                    {
                        return;
                    }

                    window.prevent_default();
                    on_hold(event.start_position, window, cx);
                });
            },
        )
        .size_full(),
    )
}

#[cfg(test)]
#[path = "hold.test.rs"]
mod tests;
