mod list;
mod view;

use crate::shared::ui::views::reorder;

pub use list::{items, list};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    Song,
    Video,
}

impl Shape {
    pub fn height(self) -> f32 {
        match self {
            Self::Song => reorder::ROW,
            Self::Video => 56.0,
        }
    }
}
