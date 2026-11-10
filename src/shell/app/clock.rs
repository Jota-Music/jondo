use std::time::{Duration, Instant};

use gpui::{Context, Window};

use crate::shared::ui::views::design::theme;

use super::*;
use crate::shared::platform::pin;

impl Jondo {
    pub(super) fn update_accent(&mut self, cx: &gpui::App) {
        let cover = self
            .player
            .read(cx)
            .current()
            .and_then(|song| song.cover.clone());

        let color = cover.as_deref().and_then(|url| {
            self.covers.request(url);
            self.covers.color(url)
        });

        if let Some(color) = color {
            if self.accent_cover != cover {
                self.accent = color;
                self.binary = crate::shared::ui::views::design::color::contrast(color);
                self.accent_cover = cover;
            }
        } else if cover.is_none() && self.accent_cover.is_some() {
            self.accent = theme::ACCENT;
            self.binary = theme::BINARY;
            self.accent_cover = None;
        }
    }

    pub fn needs_frames(&self, cx: &gpui::App) -> bool {
        self.player.read(cx).playing || self.animates(cx)
    }

    pub(super) fn animates(&self, cx: &gpui::App) -> bool {
        let player = self.player.read(cx);
        // The sheets are not here on purpose: a sheet requests its own frames
        // while it moves and marks them as motion, so the clock waking every
        // 16ms for the same slide would only add a pump of network, events and
        // cover polls per frame of it.
        player.loading
            || player.seek_preview.is_some()
            || self.preview.playing
            || self.preview.pending
            || self.ui.drag_scroll
            || self
                .toast
                .as_ref()
                .is_some_and(crate::shell::domain::ui::Toast::animating)
            || self.menu_closing.is_some()
            || self.ui.search_closing.is_some()
            || self.editor.waiting()
            || self.editor.animating()
            || self.clipboard.busy
            || crate::shared::ui::views::components::hover::running()
    }

    pub(super) fn pace(&self, cx: &gpui::App) -> Duration {
        if self.animates(cx) {
            FRAME
        } else if self.player.read(cx).playing {
            if self.disc_shown { CLOCK } else { IDLE }
        } else if self.covers.pending() {
            COVER
        } else {
            IDLE
        }
    }

    pub fn modal(&self, cx: &gpui::App) -> bool {
        self.ui.create_open
            || self.ui.rename.is_some()
            || self.ui.confirm.is_some()
            || self.ui.removing.is_some()
            || self.ui.picker.is_some()
            || self.editor.open
            || self.rooms_ui.confirm.is_some()
            || self.rooms.read(cx).show
            || self.ui.backup_sheet
    }

    pub fn pump(&mut self, cx: &mut gpui::Context<Self>) -> bool {
        let mut changed = self.covers.poll();
        changed |= self.drain_events(cx);
        if self
            .clipboard
            .pending
            .as_ref()
            .is_some_and(|link| self.clipboard_redundant(link, cx))
        {
            self.clipboard.pending = None;
            changed = true;
        }

        let preview = &mut self.preview;
        self.player.update(cx, |player, _| preview.tick(player));
        changed |= self.sync_sessions(cx);
        changed |= self.tick_spotify(cx);
        self.tick_presence(cx);
        #[cfg(not(target_os = "android"))]
        {
            changed |= self.tick_backup(cx);
        }

        self.room_tick(cx);

        if self.player.update(cx, |player, _| player.tick()) {
            changed = true;
            self.next(cx);
        }

        if self.player.update(cx, |player, _| player.settle()) {
            changed = true;
        }

        self.update_accent(cx);

        if let Some(toast) = &self.toast
            && toast.at.elapsed() > Duration::from_secs(4)
        {
            self.toast = None;
            changed = true;
        }

        changed
    }

    pub fn tick(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        // Base reads the desktop's reduced-motion preference into the app flag
        // once it starts. The product's transitions are part of its language,
        // so the clock takes the flag back instead of letting every render do
        // it: a frame is never the owner of application state.
        if cx.reduce_motion() {
            cx.set_reduce_motion(false);
        }

        for frame in self.covers.stale() {
            window.drop_image(frame).ok();
        }

        for image in self.covers.dropped() {
            window.drop_image(image).ok();
        }

        let pumped = self.pump(cx);
        if pumped {
            // The pump moved real data, not just chrome: force the page back
            // into the frame even if a shell-only notify marked it as motion.
            crate::shell::app::motion::clear();
            cx.notify();
        }

        self.reorder_tick(cx);
        #[cfg(target_os = "android")]
        let android = self.tick_android(cx);
        #[cfg(target_os = "android")]
        if android {
            cx.notify();
        }

        crate::music::track::views::youtube::tick(self, window, cx);

        if self.route != *self.history.current() {
            self.route = self.history.current().clone();
            self.route_changed_at = Some(Instant::now());
            self.paint_page(cx);
        }

        // The vinyl only exists while the current layout shows it, and the
        // clock owns that decision: render must not carry application state.
        let narrow = window.viewport_size().width < theme::unit(768.0);
        let has_song = self.player.read(cx).current().is_some();
        self.disc_shown =
            has_song && ((!narrow && !self.ui.compact) || (narrow && self.ui.player_open));

        if !self.pin_applied {
            self.pin_applied = true;
            if self.store.always_on_top && pin::pinnable() {
                pin::set(true);
            }
        }

        let now = Instant::now();
        let dt = (now - self.disc_at).as_secs_f32().clamp(0.0, 0.25);
        self.disc_at = now;
        if self.player.read(cx).playing {
            self.disc_spin = (self.disc_spin + dt * std::f32::consts::TAU / SPIN_PERIOD)
                .rem_euclid(std::f32::consts::TAU);
        }

        #[cfg(target_os = "android")]
        if self.animates(cx) {
            cx.notify();
        }

        // Nothing changed: this frame only advances motion, so the page under
        // it keeps the frame it already painted.
        if !pumped && self.needs_frames(cx) {
            crate::shell::app::motion::note();
        }

        let busy = pumped || self.needs_frames(cx);
        #[cfg(target_os = "android")]
        let busy = busy || android;

        busy
    }

    pub(super) fn menu_key(
        &mut self,
        event: &gpui::KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        crate::shell::views::menu::keys(self, event.keystroke.key.as_str(), window, cx);
    }
}
