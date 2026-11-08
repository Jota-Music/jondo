use std::collections::{HashSet, VecDeque};

use crate::shared::kernel::app::{Load, Pending};
use crate::shared::music::domain::{
    Album, ArtistView, ChannelView, Playlist, SpotifySearch, UserView,
};

/// What discovery keeps: the search, the entities a page opens by id, and the
/// playlist cache the player and the lists read.
#[derive(Default)]
pub struct Catalog {
    pub query: String,
    pub search: Load<SpotifySearch>,
    /// The query whose search already failed, so a page does not retry it on
    /// every frame.
    pub failed: Option<String>,
    /// A query typed before the Spotify session finished connecting.
    pub pending: Option<String>,
    pub playlists: Pending<String, Playlist>,
    pub albums: Pending<String, Album>,
    pub artists: Pending<String, ArtistView>,
    pub users: Pending<String, UserView>,
    pub channels: Pending<String, ChannelView>,
    pub discography: HashSet<(String, String)>,
    pub discography_loading: HashSet<(String, String)>,
    pub recent: VecDeque<String>,
}

impl Catalog {
    pub fn results(&self) -> Option<&SpotifySearch> {
        self.search.value.as_ref()
    }

    pub fn begin(&mut self, query: &str) {
        self.query = query.to_owned();
        self.search = Load::default();
        self.search.begin();
    }
}
