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
const CLOSE: f32 = 96.0;
const FLING: f32 = 500.0;
const BAR: f32 = theme::PLAYER_BAR;
const CLOSE_TOP: f32 = 12.0;
const CLOSE_SIZE: f32 = 32.0;
