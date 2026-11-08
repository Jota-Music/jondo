mod actions;
mod editor;
mod page;
mod preview;
mod results;
#[cfg(test)]
#[path = "youtube.test.rs"]
mod tests;

use std::time::Duration;

pub use crate::music::track::app::Editor;
pub use crate::music::track::app::Preview;
pub use crate::music::track::domain::Mode;
pub use actions::{changed, parse_video, save, stripped, switch, video_of, wire};
pub use editor::show;
pub use page::{edit, open, tick};

use page::submit;

const LINK_DELAY: Duration = Duration::from_millis(300);
const SEARCH_DELAY: Duration = Duration::from_millis(350);
