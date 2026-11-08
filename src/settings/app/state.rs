use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::settings::app::update;
use crate::settings::domain::SpotifyState;
use crate::shared::discord::Presence;
use crate::shared::spotify;

/// Everything the settings panel drives: the two accounts, Discord presence,
/// the cookie browser list and the release check.
pub struct Settings {
    pub spotify: SpotifyState,
    pub user: String,
    pub error: Option<String>,
    pub auth: Option<Arc<spotify::Session>>,
    pub retry: Option<Instant>,
    pub backoff: Duration,
    pub applied: String,
    pub youtube_applied: String,
    pub youtube_expired: bool,
    pub youtube_rescue: bool,
    pub youtube_load: u64,
    pub discord: Presence,
    pub discord_key: Option<(String, bool)>,
    pub discord_on: bool,
    pub browsers: Vec<&'static str>,
    pub browsers_checked: bool,
    pub browsers_loading: bool,
    pub update: Option<update::Info>,
    pub update_checked: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            spotify: SpotifyState::Off,
            user: String::new(),
            error: None,
            auth: None,
            retry: None,
            backoff: crate::shell::app::SPOTIFY_RETRY,
            applied: String::new(),
            youtube_applied: String::new(),
            youtube_expired: false,
            youtube_rescue: false,
            youtube_load: 0,
            discord: Presence::new(),
            discord_key: None,
            discord_on: false,
            browsers: Vec::new(),
            browsers_checked: false,
            browsers_loading: false,
            update: None,
            update_checked: false,
        }
    }
}
