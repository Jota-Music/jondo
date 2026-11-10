use std::time::Instant;

use crate::shared::kernel::app::Load;
use crate::shared::music::domain::{Channel, Playlist, Song};

pub struct Toast {
    pub text: String,
    pub error: bool,
    pub at: Instant,
}

impl Toast {
    /// The toast only moves while it fades in or out; between those the clock
    /// can stay idle instead of repainting the whole app for four seconds.
    pub fn animating(&self) -> bool {
        let age = self.at.elapsed().as_secs_f32();
        age < 0.2 || age > 3.8
    }
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
