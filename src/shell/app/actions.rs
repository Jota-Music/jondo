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

/// Focus context the application binds its global commands to. Printable keys
/// also exclude the `Input` context, so typing in a field never reaches them.
pub(super) const CONTEXT: &str = "Jondo";

pub(super) fn bindings() -> Vec<KeyBinding> {
    vec![
        KeyBinding::new("escape", Dismiss, Some(CONTEXT)),
        KeyBinding::new("space", TogglePlay, Some("Jondo && !Input")),
        KeyBinding::new("shift-right", NextTrack, Some(CONTEXT)),
        KeyBinding::new("shift-left", PreviousTrack, Some(CONTEXT)),
        KeyBinding::new("right", SeekForward, Some(CONTEXT)),
        KeyBinding::new("left", SeekBackward, Some(CONTEXT)),
        KeyBinding::new("m", ToggleMute, Some("Jondo && !Input")),
        KeyBinding::new("q", ToggleQueue, Some("Jondo && !Input")),
    ]
}

impl Jondo {
    /// A panel, a sheet or the search box owns the keyboard while it is open.
    fn commandable(&self, cx: &gpui::App) -> bool {
        !self.modal(cx) && !self.ui.search_open
    }

    pub(super) fn dismiss_overlays(&mut self, _: &Dismiss, _: &mut Window, cx: &mut Context<Self>) {
        // The Android back gesture arrives as Escape; with nothing left to
        // dismiss it has to step through the history like the header arrow.
        let overlay = self.menu.is_some()
            || self.modal(cx)
            || self.ui.player_open
            || self.ui.queue_open
            || self.ui.search_open;
        let selected = !self.selection.is_empty();

        if self.menu.is_some() {
            self.close_menu();
            if !self.selection.is_empty() {
                self.clear_selection();
            }

            cx.notify();
            return;
        }

        if self.editor.open {
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

        if !self.selection.is_empty() {
            self.clear_selection();
        }

        if !overlay && !selected {
            if self.history.can_back() {
                self.back(cx);
            } else {
                #[cfg(target_os = "android")]
                crate::shared::platform::android::leave::background();
            }
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
        self.notify_shell(cx);
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
        self.notify_shell(cx);
    }

    pub(super) fn mute_toggle(&mut self, _: &ToggleMute, _: &mut Window, cx: &mut Context<Self>) {
        if !self.commandable(cx) {
            return;
        }

        self.toggle_mute(cx);
        self.notify_shell(cx);
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

#[cfg(test)]
#[path = "actions.test.rs"]
mod tests;
