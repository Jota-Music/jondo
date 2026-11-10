mod drag;
mod header;
mod panel;
mod state;
#[cfg(test)]
#[path = "sheet.test.rs"]
mod tests;

use std::time::Duration;

use crate::shared::ui::views::design::theme;

pub use header::{Header, header};
pub use panel::Sheet;

pub const DURATION: Duration = Duration::from_millis(300);
const EXIT: Duration = Duration::from_millis(250);
/// The shortest a release can take to put the sheet back. Anything faster
/// reads as a snap, not as the hand coming to a stop.
const MIN_SETTLE: f32 = 0.1;
const CLOSE: f32 = 96.0;
const FLING: f32 = 500.0;
const BAR: f32 = theme::PLAYER_BAR;
const CLOSE_TOP: f32 = 12.0;
const CLOSE_SIZE: f32 = 32.0;
