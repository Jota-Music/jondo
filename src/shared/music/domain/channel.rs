use super::{Playlist, Song};

#[derive(Clone, Debug, Default)]
pub struct Channel {
    pub id: String,
    pub name: String,
    pub avatar: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct ChannelView {
    pub id: String,
    pub name: String,
    pub avatar: Option<String>,
    pub videos: Vec<Song>,
    pub playlists: Vec<Playlist>,
}
