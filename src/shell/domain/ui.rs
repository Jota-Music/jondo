use std::time::Instant;

use crate::shared::kernel::app::Load;
use crate::shared::music::domain::{Channel, Playlist, Song};

pub struct Toast {
    pub text: String,
    pub error: bool,
    pub at: Instant,
}

#[derive(Default)]
pub struct SearchState {
    pub query: String,
    pub songs: Load<Vec<Song>>,
    pub channels: Load<Vec<Channel>>,
    pub playlists: Load<Vec<Playlist>>,
    pub scroll_top: bool,
    pub tab: String,
    pub list: bool,
}
