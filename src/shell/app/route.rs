use std::time::Instant;

use crate::shared::i18n::t;
use crate::shared::music::domain::Song;
use crate::shared::spotify;
use crate::shell::domain::Route;
use crate::shell::domain::link::{Link, Target};

use super::*;

impl Jondo {
    pub fn open(&mut self, route: Route, cx: &mut gpui::Context<Self>) {
        self.close_player();
        self.close_queue();
        self.clear_selection();
        if !matches!(route, Route::Home) {
            self.ui.sort_mode = false;
        }

        match &route {
            Route::Playlist(id) => {
                self.catalog
                    .update(cx, |catalog, _| catalog.playlists.remove(id));
                self.ensure_playlist(id, cx);
            }
            Route::Album(id) => {
                self.catalog
                    .update(cx, |catalog, _| catalog.albums.remove(id));
                self.ensure_album(id, cx);
            }
            Route::Artist(id) => {
                self.catalog
                    .update(cx, |catalog, _| catalog.artists.remove(id));
                self.catalog.update(cx, |catalog, _| {
                    catalog.discography.retain(|(artist_id, _)| artist_id != id)
                });
                self.ensure_artist(id, cx);
            }
            Route::User(username) => {
                self.catalog
                    .update(cx, |catalog, _| catalog.users.remove(username));
                self.ensure_user(username, cx);
            }
            Route::Channel(id) => {
                self.catalog
                    .update(cx, |catalog, _| catalog.channels.remove(id));
                self.ui.channel_tab = "videos".to_owned();
                self.ui.channel_query.clear();
                self.ui.channel_order = Order::Added;
                self.ensure_channel(id, cx);
            }
            Route::Catalog(kind, query) => {
                if kind == &SearchType::Track {
                    if self.catalog.read(cx).failed.as_deref() != Some(query) {
                        self.catalog.update(cx, |catalog, _| catalog.failed = None);
                    }

                    self.ensure_catalog(query, cx);
                }
            }
            Route::Following => self.ensure_following(cx),
            Route::Rooms => {
                if !self.rooms.read(cx).relay.trim().is_empty()
                    && self.rooms.read(cx).checked_at.is_none()
                    && self.rooms.read(cx).check_due.is_none()
                    && !self.rooms.read(cx).checking
                {
                    self.rooms.update(cx, |rooms, _| rooms.schedule_check());
                }

                self.ensure_room_statuses(cx)
            }
            Route::Settings if !self.rooms.read(cx).relay.trim().is_empty() => {
                self.rooms.update(cx, |rooms, _| rooms.schedule_check())
            }
            Route::Settings => {}
            _ => {}
        }

        self.history.open(route);
        self.route = self.history.current().clone();
        self.route_changed_at = Some(Instant::now());
    }

    pub fn back(&mut self) {
        if self.history.back() {
            self.clear_selection();
            self.route = self.history.current().clone();
            self.route_changed_at = Some(Instant::now());
        }
    }

    pub fn forward(&mut self) {
        if self.history.forward() {
            self.clear_selection();
            self.route = self.history.current().clone();
            self.route_changed_at = Some(Instant::now());
        }
    }

    pub fn open_player(&mut self) {
        self.ui.player_open = true;
        self.ui.player_closing = None;
        self.close_queue();
    }

    pub fn close_player(&mut self) {
        if self.ui.player_open {
            self.ui.player_open = false;
            self.ui.player_closing = Some(Instant::now());
        }
    }

    pub fn close_menu(&mut self) {
        if let Some(menu) = &self.ui.menu
            && self.ui.menu_closing.is_none()
        {
            self.ui.menu_closing = Some((menu.stamp, Instant::now()));
        }
    }

    pub fn close_editor(&mut self, cx: &mut gpui::Context<Self>) {
        let preview = &mut self.preview;
        self.player.update(cx, |player, _| preview.stop(player));
        if self.ui.closing.is_none() {
            self.ui.closing = Some(Instant::now());
        }
    }

    pub fn open_queue(&mut self, cx: &gpui::App) {
        self.clear_selection();
        let at = self.player.read(cx).queue.at;
        self.ui.queue_open = true;
        self.ui.queue_closing = None;
        self.scroll("queue")
            .scroll_to_item(at.saturating_sub(1), gpui::ScrollStrategy::Top);
        self.close_player();
    }

    pub fn close_queue(&mut self) {
        if self.ui.queue_open {
            self.clear_selection();
            self.ui.queue_open = false;
            self.ui.queue_closing = Some(Instant::now());
            self.ui.drag_queue_from = None;
            self.ui.drag_queue_to = None;
            self.ui.pressed_queue = None;
            if self.ui.reorder == Some(Reorder::Queue) {
                self.ui.reorder = None;
                self.ui.drag_pointer = None;
                self.ui.drag_scroll = false;
            }
        }
    }

    pub fn open_search(&mut self) {
        self.ui.search_open = true;
        self.ui.search_focus = true;
    }

    pub fn select_single(&mut self, id: &str, index: usize) {
        self.ui.selection = vec![id.to_owned()];
        self.ui.anchor = Some(index);
    }

    pub fn select_toggle(&mut self, id: &str, index: usize) {
        if let Some(position) = self.ui.selection.iter().position(|item| item == id) {
            self.ui.selection.remove(position);
        } else {
            self.ui.selection.push(id.to_owned());
        }

        self.ui.anchor = if self.ui.selection.is_empty() {
            None
        } else {
            Some(index)
        };
    }

    pub fn select_all(&mut self, songs: &[Song]) {
        for song in songs {
            if !self.ui.selection.iter().any(|id| id == &song.id) {
                self.ui.selection.push(song.id.clone());
            }
        }
    }

    pub fn enqueue(&mut self, songs: Vec<Song>, cx: &mut gpui::Context<Self>) {
        if songs.is_empty() {
            return;
        }

        // Adds land right after the playing track, so they are the next to
        // play, and an empty queue starts playing them.
        let (idle, at) = {
            let player = self.player.read(cx);
            let idle = player.queue.songs.is_empty();
            (idle, if idle { 0 } else { player.queue.at + 1 })
        };
        self.player
            .update(cx, |player, _| player.queue.insert(songs, at));
        if idle {
            self.play_current(cx);
        }

        self.persist(cx);
    }

    pub fn select_range(&mut self, ids: &[String], anchor: usize, index: usize) {
        let start = anchor.min(index);
        let end = anchor.max(index);
        self.ui.selection = ids
            .get(start..=end)
            .map(|slice| slice.to_vec())
            .unwrap_or_default();
    }

    pub fn clear_selection(&mut self) {
        self.ui.selection.clear();
        self.ui.anchor = None;
    }

    pub fn open_clipboard(&mut self, cx: &mut gpui::Context<Self>) {
        if self.clipboard.busy {
            return;
        }

        let Some(target) = self
            .clipboard
            .pending
            .as_ref()
            .map(|link| link.target.clone())
        else {
            return;
        };

        match target {
            Target::Route(route) => {
                self.clipboard.pending = None;
                self.open(route, cx);
            }
            Target::Song(id) => self.play_clipboard(id, cx),
        }
    }

    pub(super) fn play_clipboard(&mut self, id: String, cx: &mut gpui::Context<Self>) {
        if let Some(track) = id.strip_prefix("spotify:track:") {
            let Some(session) = self.settings.read(cx).auth.clone() else {
                self.fail(t("Connect Spotify to play this song"));
                return;
            };

            self.clipboard.busy = true;
            let http = self.http.clone();
            let track = track.to_owned();
            self.jobs.spawn(async move {
                let result = spotify::search(&http, &session, &track)
                    .await
                    .and_then(|search| {
                        search
                            .tracks
                            .into_iter()
                            .find(|song| !song.broken)
                            .ok_or_else(|| "Spotify could not open this track".to_owned())
                    });
                Event::ClipboardSong(result)
            });
            return;
        }

        self.clipboard.busy = true;
        let video = id.strip_prefix("youtube:").unwrap_or(&id).to_owned();
        let youtube = self.youtube.clone();
        self.jobs.spawn(async move {
            let result = youtube.video(&video).await.map(|video| video.song());
            Event::ClipboardSong(result)
        });
    }

    pub fn dismiss_clipboard(&mut self) {
        self.clipboard.dismiss();
    }

    pub fn remember_clipboard(&mut self, text: &str) {
        self.clipboard.remember(text);
    }

    pub fn clipboard_link(&self, cx: &gpui::App) -> Option<&Link> {
        self.clipboard
            .pending
            .as_ref()
            .filter(|link| !self.clipboard_redundant(link, cx))
    }

    pub(super) fn clipboard_redundant(&self, link: &Link, cx: &gpui::App) -> bool {
        match &link.target {
            Target::Route(route) => &self.route == route,
            Target::Song(id) => {
                let player = self.player.read(cx);
                player.current().is_some_and(|song| &song.id == id)
                    && (player.playing || player.loading)
            }
        }
    }
}
