mod catalogs;
mod execute;
mod item;
mod rows;
mod sheet;
#[cfg(test)]
#[path = "menu.test.rs"]
mod tests;
mod view;

use std::time::Duration;

pub use crate::shell::domain::Command;
pub use catalogs::{channel_order, languages, playlist_order, search_types};
pub use item::Item;
pub use view::{Menu, keys, show};

const WIDTH: f32 = 224.0;
const ROW: f32 = 40.0;
const CHOICE_WIDTH: f32 = 216.0;
const CHOICE_ROW: f32 = 32.0;
const GAP: f32 = 6.0;
const HEADER: f32 = 33.0;
const OPEN: Duration = Duration::from_millis(140);
const EXIT: Duration = Duration::from_millis(160);
