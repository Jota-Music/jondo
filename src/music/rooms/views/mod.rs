mod badges;
mod card;
mod controls;
mod join;
mod page;

pub use page::{overlay, show, wire};

#[derive(Default)]
pub struct State {
    pub confirm: Option<String>,
    pub pressed: Option<String>,
    pub drag_from: Option<String>,
    pub drag_to: Option<String>,
}

/// The row for the room the session is in while it is not saved yet, so it can
/// be saved from the shelf without retyping its code.
const ACTIVE_ROOM: &str = "active-room";
