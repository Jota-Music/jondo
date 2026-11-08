use crate::shared::kernel::app::Load;
use crate::shared::music::domain::{Follow, Playlist};

/// What the user keeps: the account's rootlist, its follows and the YouTube
/// subscriptions, plus the state of the two fetches that fill them.
#[derive(Default)]
pub struct Library {
    pub spotify: Vec<Playlist>,
    pub youtube: Vec<Playlist>,
    /// The Spotify rootlist fetch.
    pub load: Load<()>,
    /// The account's follows, as opposed to the ones the user added locally.
    pub follows: Vec<Follow>,
    pub friends: Vec<String>,
    /// The following fetch.
    pub following: Load<()>,
}
