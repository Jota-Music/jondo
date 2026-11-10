use gpui::{Context, InteractiveElement, ParentElement, Styled, Window};

use super::*;

impl gpui::Render for Jondo {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        crate::shared::ui::views::design::theme::follow(window);
        // The cover frame advances with the page (`Screen::render`), not here:
        // a shell frame that skips the page must not age the covers it paints.
        // The clock owns every state change; render only wakes it when motion
        // starts, so the first animation frame does not wait for the idle poll.
        // Covers are polled on the clock too: this render may have just asked
        // for the first one, and waiting out an idle sleep would leave it
        // blank for that long.
        if self.animates(cx) || self.covers.pending() {
            self.clock.notify_one();
        }

        gpui::div()
            .size_full()
            .key_context(actions::CONTEXT)
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::menu_key))
            .on_action(cx.listener(Self::dismiss_overlays))
            .on_action(cx.listener(Self::play_pause))
            .on_action(cx.listener(Self::next_track))
            .on_action(cx.listener(Self::previous_track))
            .on_action(cx.listener(Self::seek_forward))
            .on_action(cx.listener(Self::seek_backward))
            .on_action(cx.listener(Self::mute_toggle))
            .on_action(cx.listener(Self::queue_toggle))
            .child(crate::shell::views::layouts::show(self, window, cx))
    }
}
