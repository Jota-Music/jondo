mod actions;
mod clock;
mod drag;
mod events;
mod jobs;
#[cfg(target_os = "android")]
pub(crate) mod platform;
mod render;
mod route;
mod setup;
mod ui;

pub mod clipboard;

#[cfg(test)]
#[path = "app.test.rs"]
mod tests;

pub use crate::settings::app::SpotifyState;
pub use crate::shared::music::app::{Cache, Table};
pub use crate::shared::music::domain::{Filter, Order, SearchType};
pub use crate::shared::platform::pin::pinnable;
pub use crate::shell::domain::{Choice, Drag, Reorder};
pub use crate::shell::domain::{History, Route};
pub use crate::shell::domain::{SearchState, Toast};
pub use jobs::{Event, Jobs};
#[cfg(target_os = "android")]
pub use ui::PendingSaf;
pub use ui::UiState;

pub(crate) use crate::shared::storage::library as cache;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui::{Entity, Rgba, ScrollHandle, UniformListScrollHandle};
use gpui_kit::component::input::{InputState, TextareaState};

use crate::music::catalog::Catalog;
use crate::music::library::Library;
use crate::music::player::Player;
use crate::music::rooms::Rooms;
use crate::settings::Settings;
use crate::shared::cover::Covers;
use crate::shared::storage::Store;
use crate::shared::youtube::Youtube;
use crate::shell::app::clipboard::Clipboard;

pub(crate) const IN_MEMORY_PLAYLISTS: usize = 4;
/// One frame per display refresh while something moves.
const FRAME: Duration = Duration::from_millis(16);
/// The playback clock: the disc steps a little under ten times a second, so
/// painting faster only rebuilds the same frame.
const CLOCK: Duration = Duration::from_millis(96);
/// How long the idle clock sleeps between looks at the app.
const IDLE: Duration = Duration::from_millis(250);
const SPIN_PERIOD: f32 = 120.0;
pub(crate) const SYNC_DEBOUNCE: Duration = Duration::from_secs(3);
pub(crate) const SYNC_MAX_WAIT: Duration = Duration::from_secs(20);
pub(crate) const SYNC_WATCH: Duration = Duration::from_secs(10);
pub(crate) const SPOTIFY_RETRY: Duration = Duration::from_secs(15);
pub(crate) const SPOTIFY_RETRY_MAX: Duration = Duration::from_secs(5 * 60);

pub struct Jondo {
    pub(crate) focus: gpui::FocusHandle,
    pub(crate) fields: HashMap<&'static str, Entity<InputState>>,
    pub(crate) textareas: HashMap<&'static str, Entity<TextareaState>>,
    pub(crate) scrolls: HashMap<String, UniformListScrollHandle>,
    pub(crate) panes: HashMap<String, ScrollHandle>,
    pub(crate) subscriptions: Vec<gpui::Subscription>,
    pub(crate) route: Route,
    pub(crate) route_key: gpui::SharedString,
    pub(crate) history: History,
    pub(crate) store: Store,
    pub(crate) covers: Covers,
    pub(crate) player: Entity<Player>,
    pub(crate) preview: crate::music::track::app::Preview,
    pub(crate) jobs: Jobs,
    pub(crate) youtube: Arc<Youtube>,
    pub(crate) http: reqwest::Client,
    pub(crate) catalog: Entity<Catalog>,
    pub(crate) library: Entity<Library>,
    pub(crate) settings: Entity<Settings>,
    pub(crate) probes: HashSet<String>,
    pub(crate) search: SearchState,
    pub(crate) ui: UiState,
    pub(crate) clipboard: Clipboard,
    pub(crate) rooms: Entity<Rooms>,
    pub(crate) rooms_ui: crate::music::rooms::views::State,
    pub(crate) accent: Rgba,
    pub(crate) binary: Rgba,
    pub(crate) accent_cover: Option<String>,
    pub(crate) disc_spin: f32,
    disc_at: Instant,
    pin_applied: bool,
    /// Wakes the clock out of its idle sleep when motion starts, so the first
    /// animation frame does not wait for the next idle look.
    clock: Arc<tokio::sync::Notify>,
    pub(crate) toast: Option<Toast>,
    pub(crate) sync_first: Option<Instant>,
    pub(crate) sync_armed: Option<Instant>,
    pub(crate) sync_watch: Option<Instant>,
    /// A background backup owns the store snapshot until it answers.
    #[cfg(not(target_os = "android"))]
    pub(crate) backup_running: bool,
    /// Bumped on every save, so a stale backup result is discarded.
    pub(crate) store_revision: u64,
    #[cfg(target_os = "android")]
    pub(crate) saf: crate::shared::platform::saf::Saf,
    #[cfg(target_os = "android")]
    pub(crate) pending_saf: Option<PendingSaf>,
    pub(crate) pending_play: Option<String>,
    pub(crate) pending_album: Option<String>,
    pub(crate) pending_shuffle: Option<String>,
    pub(crate) play_token: u64,
    pub(crate) route_changed_at: Option<Instant>,
}
