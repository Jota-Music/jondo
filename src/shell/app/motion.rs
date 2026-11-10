use std::cell::Cell;

use gpui::Window;

thread_local! {
    static MOVING: Cell<bool> = const { Cell::new(false) };
}

/// Asks for the next frame on the behalf of something that only moves: a sheet
/// sliding, a menu fading, the player clock turning the vinyl. The page under
/// that motion keeps the frame it already painted instead of being rebuilt and
/// laid out again for a move it does not take part in.
pub fn ask(window: &Window) {
    note();
    window.request_animation_frame();
}

/// The same, for a frame the caller asks for with [`gpui::Context::notify`].
pub fn note() {
    MOVING.with(|moving| moving.set(true));
}

/// Whether the frame being drawn only exists to move something, consuming the
/// note so the next state change paints the page again.
pub fn took() -> bool {
    MOVING.with(|moving| moving.replace(false))
}

/// Forgets a pending shell-only frame. The clock calls it when its pump saw a
/// real data change, so a notify that only touched the chrome cannot hide the
/// page update that arrived in the same frame.
pub fn clear() {
    MOVING.with(|moving| moving.set(false));
}
