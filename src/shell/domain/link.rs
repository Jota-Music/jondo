use crate::shell::domain::Route;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Playlist,
    Album,
    Artist,
    User,
    Channel,
    Song,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    Route(Route),
    Song(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link {
    pub kind: Kind,
    pub text: String,
    pub target: Target,
}
