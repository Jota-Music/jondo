use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui::{
    App, AppContext as _, Context, Entity, Focusable as _, ScrollHandle, UniformListScrollHandle,
    Window,
};
use gpui_kit::component::input::{InputState, TextareaState};

use crate::music::player::Repeat;
use crate::music::player::app::Player;
use crate::music::rooms::Rooms;
use crate::shared::cover::Covers;
use crate::shared::i18n::{self, t};
use crate::shared::music::domain::Source;
use crate::shared::storage::Store;
use crate::shared::ui::views::design::theme;
use crate::shared::youtube::Youtube;
use crate::shell::app::clipboard::Clipboard;

use super::*;

impl Jondo {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self::boot(Store::load(), window, cx)
    }

    /// Builds the app on a given store. The tests hand it one backed by a
    /// temporary home, so a run never reads or writes the real state.
    pub fn boot(mut store: Store, window: &mut Window, cx: &mut Context<Self>) -> Self {
        if store.spotify_device.is_empty() {
            store.spotify_device = crate::shared::spotify::device();
        }

        let search_source = if store.spotify.is_some() {
            Source::Spotify
        } else {
            Source::YouTube
        };

        let mut player = Player::new(store.volume, store.muted);
        // The saved order is the play order: a shuffled queue is stored already
        // permuted, so it must not be reshuffled on every launch.
        player.queue.replace(store.queue.clone(), store.queue_index);
        player.queue.flag_shuffle(store.shuffle);
        player.repeat = crate::music::player::parse_repeat(&store.repeat).unwrap_or(Repeat::Off);
        player.load_position(store.position);
        if let Some(song) = player.current() {
            player.duration = song.duration as f64;
        }

        let spotify_playlists = store
            .spotify
            .as_ref()
            .map(|token| cache::library(&token.username))
            .unwrap_or_default();

        let mut rooms = Rooms::new(
            store.relay_url.clone(),
            store.relay_token.clone(),
            store.room_code.clone(),
            store.room_password.clone(),
        );
        if let Ok(url) = std::env::var("RELAY_API_URL") {
            let url = url.trim().to_owned();
            if !url.is_empty() {
                rooms.relay = url;
                rooms.token = std::env::var("RELAY_API_TOKEN")
                    .map(|token| token.trim().to_owned())
                    .unwrap_or_default();
                rooms.relay_locked = true;
            }
        }

        if !rooms.relay.trim().is_empty() {
            rooms.schedule_check();
        }
        let rooms = cx.new(|_| rooms);

        let library = cx.new(|_| Library {
            spotify: spotify_playlists,
            ..Default::default()
        });
        let catalog = cx.new(|_| Catalog::default());
        let player = cx.new(|_| player);
        let settings = cx.new(|_| Settings::default());
        let seen = cx.entity().downgrade();
        let screen = cx.new(|cx| crate::shell::views::layouts::Screen::new(seen, cx));

        let mut app = Self {
            focus: cx.focus_handle(),
            fields: HashMap::new(),
            textareas: HashMap::new(),
            scrolls: HashMap::new(),
            panes: HashMap::new(),
            pane_offsets: HashMap::new(),
            subscriptions: Vec::new(),
            route: Route::Home,
            history: History::new(),
            covers: Covers::new(),
            screen,
            player,
            preview: crate::music::track::app::Preview::default(),
            jobs: Jobs::new(),
            youtube: Arc::new(Youtube::new()),
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(20))
                .build()
                .unwrap_or_default(),
            catalog,
            library,
            settings,
            probes: HashSet::new(),
            search: SearchState::default(),
            menu: None,
            menu_closing: None,
            selection: Vec::new(),
            anchor: None,
            editor: crate::music::track::app::Editor::default(),
            clipboard: Clipboard::default(),
            rooms,
            rooms_ui: Default::default(),
            ui: UiState {
                compact: store.compact,
                search_source,
                ..Default::default()
            },
            accent: theme::ACCENT,
            binary: theme::BINARY,
            accent_cover: None,
            disc_spin: 0.0,
            disc_shown: false,
            disc_at: Instant::now(),
            pin_applied: false,
            clock: Arc::new(tokio::sync::Notify::new()),
            toast: None,
            sync_first: None,
            sync_armed: None,
            sync_watch: None,
            #[cfg(not(target_os = "android"))]
            backup_running: false,
            store_revision: 0,
            #[cfg(target_os = "android")]
            saf: crate::shared::platform::saf::Saf::new(),
            #[cfg(target_os = "android")]
            pending_saf: None,
            pending_play: None,
            pending_album: None,
            pending_shuffle: None,
            play_token: 0,
            route_changed_at: None,
            store,
        };

        i18n::set(if app.store.locale.is_empty() {
            i18n::Locale::detect()
        } else {
            i18n::Locale::parse(&app.store.locale).unwrap_or(i18n::Locale::En)
        });
        app.settings.update(cx, |settings, _| {
            settings.youtube_applied = app.store.youtube_cookies.clone()
        });
        let applied = app
            .store
            .spotify
            .as_ref()
            .map(|auth| auth.data.clone())
            .unwrap_or_default();
        app.settings
            .update(cx, |settings, _| settings.applied = applied);
        app.youtube.set_session(&app.store.youtube_cookies);
        // Only a session that will play something soon pays the scrape: a fresh
        // home stays offline until the user asks for audio.
        if app.player.read(cx).current().is_some() || app.youtube.signed_in() {
            let youtube = app.youtube.clone();
            app.jobs.run(async move { youtube.warm().await });
        }
        app.ui.browser_pick = app.store.youtube_browser.clone();
        app.refresh_browser(false, cx);
        app.ensure_update(cx);
        if !app.store.youtube_cookies.is_empty() && app.store.youtube_browser.is_empty() {
            app.rescue(cx);
        }

        if let Some(song) = app.player.read(cx).current().cloned() {
            app.identify(&song);
        }

        if app.youtube.signed_in() {
            app.load_youtube_library(cx);
        }

        app.resume_spotify(cx);
        #[cfg(not(target_os = "android"))]
        app.start_backup(cx);

        app.wire(window, cx);
        crate::music::library::views::playlists::wire(&mut app, window, cx);
        crate::music::rooms::views::wire(&mut app, window, cx);
        crate::settings::views::wire(&mut app, window, cx);
        crate::music::library::views::playlist::wire(&mut app, window, cx);
        crate::music::track::views::youtube::wire(&mut app, window, cx);

        cx.bind_keys(actions::bindings());
        app.focus.focus(window, cx);

        app.install_painter(cx);
        app.watch_events(cx);
        app.watch_clock(window, cx);
        app
    }

    pub fn field(
        &mut self,
        key: &'static str,
        value: &str,
        placeholder: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<InputState> {
        if let Some(entity) = self.fields.get(key).cloned() {
            return entity;
        }

        let entity = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder.to_owned()));
        if !value.is_empty() {
            entity.update(cx, |state, cx| {
                state.set_value(value.to_owned(), window, cx)
            });
        }

        self.fields.insert(key, entity.clone());
        entity
    }

    /// The search placeholder names the current source and search type, and
    /// both change from event handlers: render never syncs it.
    pub(crate) fn refresh_search_placeholder(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let placeholder = crate::shell::views::header::placeholder(self);
        if let Some(entity) = self.fields.get("search").cloned()
            && entity.read(cx).presentation().placeholder().as_str() != placeholder
        {
            entity.update(cx, |state, cx| {
                state.set_placeholder(placeholder.to_owned(), window, cx)
            });
        }
    }

    /// Focuses the search field from the action that opened or changed it,
    /// never from `render`.
    pub fn focus_search(&self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(input) = self.fields.get("search") {
            input.update(cx, |state, cx| state.focus_handle(cx).focus(window, cx));
        }
    }

    pub fn secret(
        &mut self,
        key: &'static str,
        value: &str,
        placeholder: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<InputState> {
        if let Some(entity) = self.fields.get(key) {
            return entity.clone();
        }

        let entity = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder.to_owned()));
        if !value.is_empty() {
            entity.update(cx, |state, cx| {
                state.set_value(value.to_owned(), window, cx)
            });
        }

        entity.update(cx, |state, cx| state.set_masked(true, window, cx));
        self.fields.insert(key, entity.clone());
        entity
    }

    pub fn cookies_area(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<TextareaState> {
        if let Some(entity) = self.textareas.get("cookies") {
            return entity.clone();
        }

        let entity = cx.new(|cx| {
            TextareaState::new(window, cx).placeholder(
                t("Paste cookies (must include SAPISID) or a cookies.txt export").to_owned(),
            )
        });
        self.subscriptions.push(cx.subscribe_in(
            &entity,
            window,
            |app, state, event: &gpui_kit::component::input::InputEvent, _, cx| {
                if matches!(event, gpui_kit::component::input::InputEvent::Change) {
                    app.ui.cookies_draft = state.read(cx).value().to_string();
                    cx.notify();
                }
            },
        ));
        self.textareas.insert("cookies", entity.clone());
        entity
    }

    pub fn scroll(&mut self, key: impl Into<String>) -> UniformListScrollHandle {
        self.scrolls.entry(key.into()).or_default().clone()
    }

    pub fn pane(&mut self, key: impl Into<String>) -> ScrollHandle {
        self.panes.entry(key.into()).or_default().clone()
    }

    pub fn value(&self, key: &str, cx: &App) -> String {
        self.fields
            .get(key)
            .map(|entity| entity.read(cx).value().to_string())
            .unwrap_or_default()
    }

    pub fn set_field(
        &mut self,
        key: &str,
        value: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(entity) = self.fields.get(key).cloned() {
            entity.update(cx, |state, cx| {
                state.set_value(value.to_owned(), window, cx)
            });
        }
    }

    /// Where the next search goes. An account turns the stored choice into the
    /// answer, so a search typed while the session is still connecting waits
    /// for Spotify instead of silently running on YouTube.
    pub fn live_source(&self) -> Source {
        chosen_source(self.store.spotify.is_some(), self.ui.search_source)
    }

    pub fn notify(&mut self, text: impl Into<String>) {
        self.toast = Some(Toast {
            text: text.into(),
            error: false,
            at: Instant::now(),
        });
    }

    pub fn fail(&mut self, text: impl Into<String>) {
        self.toast = Some(Toast {
            text: text.into(),
            error: true,
            at: Instant::now(),
        });
    }

    pub fn submit_search(&mut self, cx: &mut gpui::Context<Self>) {
        let query = self.ui.query.trim().to_owned();
        if query.is_empty() {
            return;
        }

        if self.live_source() == Source::Spotify {
            self.catalog_search(query, cx);
        } else {
            self.search(query, cx);
        }
    }

    /// Every input the render tree reads exists before the first frame. A view
    /// that creates an entity while it renders mutates retained state mid-frame.
    fn prepare_fields(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let search = crate::shell::views::header::placeholder(self);
        let code = self.rooms.read(cx).draft_code.clone();
        let password = self.rooms.read(cx).draft_password.clone();
        let relay = self.rooms.read(cx).relay.clone();
        let token = self.rooms.read(cx).token.clone();
        let pass = self.store.session_pass.clone();

        self.field("search", "", &search, window, cx);
        self.field("playlist-search", "", t("Search in playlist"), window, cx);
        self.field("channel-search", "", t("Search in channel"), window, cx);
        self.field("create", "", t("Playlist name"), window, cx);
        self.field("rename", "", t("Playlist name"), window, cx);
        self.field("picker-name", "", t("Playlist name"), window, cx);
        self.field("room-code", &code, "party-42", window, cx);
        self.field("youtube-link", "", t("ID or URL"), window, cx);
        self.field(
            "youtube-search",
            "",
            t("Song, artist or link..."),
            window,
            cx,
        );
        self.field("relay-url", &relay, "relay.example.com", window, cx);

        self.secret("room-password", &password, t("Optional"), window, cx);
        self.secret("relay-token", &token, "", window, cx);
        self.secret(
            "session-pass",
            &pass,
            t("Same pass on every device"),
            window,
            cx,
        );

        self.cookies_area(window, cx);
    }

    pub(super) fn wire(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.prepare_fields(window, cx);
        let search = self.field("search", "", "", window, cx);
        self.subscriptions.push(cx.subscribe_in(
            &search,
            window,
            |app, state, event: &gpui_kit::component::input::InputEvent, _, cx| match event {
                gpui_kit::component::input::InputEvent::Change => {
                    app.ui.query = state.read(cx).value().to_string();
                    // Only the header clear button reads the draft; the page
                    // under it does not change until the search is submitted.
                    app.notify_shell(cx);
                }
                gpui_kit::component::input::InputEvent::PressEnter { .. } => {
                    if app.menu.is_none() {
                        app.submit_search(cx);
                    }

                    cx.notify();
                }
                _ => {}
            },
        ));
    }

    fn watch_events(&self, cx: &mut Context<Self>) {
        let notify = self.jobs.notify.clone();
        cx.spawn(async move |this, cx| {
            loop {
                notify.notified().await;
                if this
                    .update(cx, |app, cx| {
                        app.drain_events(cx);
                        cx.notify();
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
    }

    fn watch_clock(&self, window: &Window, cx: &mut Context<Self>) {
        let clock = self.clock.clone();
        let window_handle = window.window_handle();
        cx.spawn(async move |this, cx| {
            let mut idle = 0u32;
            let mut active = false;
            loop {
                let delay = this.update(cx, |app, cx| app.pace(cx)).unwrap_or(IDLE);
                {
                    let timer = cx.background_executor().timer(delay);
                    let wake = clock.notified();
                    futures_util::pin_mut!(timer, wake);
                    futures_util::future::select(timer, wake).await;
                }

                let now_active = window_handle
                    .update(cx, |_, window, _| window.is_window_active())
                    .unwrap_or(false);
                // A paste is plausible when the window comes back, and not
                // before: waiting on a selection owner that never answers
                // freezes the frame, so there is no timer behind this read.
                if now_active
                    && !active
                    && let Ok(item) =
                        window_handle.update(cx, |_, _, app| app.read_from_clipboard())
                    && let Some(text) = item.and_then(|item| item.text())
                {
                    let _ = this.update(cx, |app, cx| {
                        if app.clipboard.detect(&text) {
                            cx.notify();
                        }
                    });
                }

                active = now_active;
                let busy = window_handle
                    .update(cx, |_, window, cx| {
                        this.update(cx, |app, cx| app.tick(window, cx)).ok()
                    })
                    .ok()
                    .flatten();
                let Some(busy) = busy else {
                    break;
                };

                if busy {
                    let _ = this.update(cx, |_, cx| cx.notify());
                }

                idle = if busy { 0 } else { idle + 1 };
                // Two minutes with no frame worth painting: the user is not
                // looking, so the decoded covers go and the heap pages go back.
                if idle >= 480 {
                    idle = 0;
                    let _ = this.update(cx, |app, cx| {
                        if !app.store.trim_memory || app.covers.pending() {
                            return;
                        }

                        app.covers.clear();
                        crate::shared::platform::memory::trim();
                        cx.notify();
                    });
                }
            }
        })
        .detach();
    }
}

/// The source the next search runs on. A stored account makes the chosen
/// switch authoritative even while the session connects, so a search typed
/// during startup is not silently answered by YouTube.
pub(super) fn chosen_source(account: bool, chosen: Source) -> Source {
    if account { chosen } else { Source::YouTube }
}
