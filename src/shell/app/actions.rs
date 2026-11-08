use gpui::{Context, KeyBinding, Window};

use super::*;

gpui::actions!(
    jondo,
    [
        Dismiss,
        TogglePlay,
        NextTrack,
        PreviousTrack,
        SeekForward,
        SeekBackward,
        ToggleMute,
        ToggleQueue,
    ]
);

/// Focus context the application binds its global commands to. Inputs keep
/// their own context, so a printable key typed in a field never reaches these.
pub(super) const CONTEXT: &str = "Jondo";

pub(super) fn bindings() -> Vec<KeyBinding> {
    vec![
        KeyBinding::new("escape", Dismiss, Some(CONTEXT)),
        KeyBinding::new("space", TogglePlay, Some(CONTEXT)),
        KeyBinding::new("shift-right", NextTrack, Some(CONTEXT)),
        KeyBinding::new("shift-left", PreviousTrack, Some(CONTEXT)),
        KeyBinding::new("right", SeekForward, Some(CONTEXT)),
        KeyBinding::new("left", SeekBackward, Some(CONTEXT)),
        KeyBinding::new("m", ToggleMute, Some(CONTEXT)),
        KeyBinding::new("q", ToggleQueue, Some(CONTEXT)),
    ]
}

impl Jondo {
    /// A panel, a sheet or the search box owns the keyboard while it is open.
    fn commandable(&self, cx: &gpui::App) -> bool {
        !self.modal(cx) && !self.ui.search_open
    }

    pub(super) fn dismiss_overlays(&mut self, _: &Dismiss, _: &mut Window, cx: &mut Context<Self>) {
        if self.ui.menu.is_some() {
            self.close_menu();
            if !self.ui.selection.is_empty() {
                self.clear_selection();
            }

            cx.notify();
            return;
        }

        if self.ui.editor.open {
            self.close_editor(cx);
        } else if self.ui.backup_sheet {
            self.ui.backup_closing = Some(std::time::Instant::now());
        } else if self.ui.create_open
            || self.ui.rename.is_some()
            || self.ui.confirm.is_some()
            || self.ui.picker.is_some()
            || self.rooms_ui.confirm.is_some()
            || self.rooms.read(cx).show
        {
            self.ui.closing = Some(std::time::Instant::now());
        } else if self.ui.player_open {
            self.close_player();
        } else if self.ui.queue_open {
            self.close_queue();
        } else if self.ui.search_open {
            self.ui.search_open = false;
            self.ui.search_closing = Some(std::time::Instant::now());
        }

        if !self.ui.selection.is_empty() {
            self.clear_selection();
        }

        cx.notify();
    }

    pub(super) fn play_pause(&mut self, _: &TogglePlay, _: &mut Window, cx: &mut Context<Self>) {
        if !self.commandable(cx) {
            return;
        }

        self.toggle(cx);
        cx.notify();
    }

    pub(super) fn next_track(&mut self, _: &NextTrack, _: &mut Window, cx: &mut Context<Self>) {
        if !self.commandable(cx) {
            return;
        }

        self.next(cx);
        cx.notify();
    }

    pub(super) fn previous_track(
        &mut self,
        _: &PreviousTrack,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.commandable(cx) {
            return;
        }

        self.previous(cx);
        cx.notify();
    }

    pub(super) fn seek_forward(&mut self, _: &SeekForward, _: &mut Window, cx: &mut Context<Self>) {
        if !self.commandable(cx) {
            return;
        }

        let (progress, duration) = {
            let player = self.player.read(cx);
            (player.progress, player.duration)
        };
        self.seek((progress + 5.0).min(duration), cx);
        cx.notify();
    }

    pub(super) fn seek_backward(
        &mut self,
        _: &SeekBackward,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.commandable(cx) {
            return;
        }

        let progress = self.player.read(cx).progress;
        self.seek((progress - 5.0).max(0.0), cx);
        cx.notify();
    }

    pub(super) fn mute_toggle(&mut self, _: &ToggleMute, _: &mut Window, cx: &mut Context<Self>) {
        if !self.commandable(cx) {
            return;
        }

        self.toggle_mute(cx);
        cx.notify();
    }

    pub(super) fn queue_toggle(&mut self, _: &ToggleQueue, _: &mut Window, cx: &mut Context<Self>) {
        if !self.commandable(cx) {
            return;
        }

        if self.ui.queue_open {
            self.close_queue();
        } else {
            self.open_queue(cx);
        }

        cx.notify();
    }
}
