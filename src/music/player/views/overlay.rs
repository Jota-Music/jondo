use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Window, div,
};

use crate::shared::ui::views::design::theme;
use crate::shell::app::Jondo;
use crate::shell::views::sheet;

use super::disc::disc;
use super::info::info;

pub fn overlay(app: &mut Jondo, window: &mut Window, cx: &mut Context<Jondo>) -> AnyElement {
    if window.viewport_size().width >= theme::unit(768.0) {
        return div().into_any_element();
    }

    if let Some(at) = app.ui.player_closing
        && at.elapsed() >= sheet::DURATION
    {
        app.ui.player_closing = None;
        cx.notify();
    }

    if !app.ui.player_open && app.ui.player_closing.is_none() {
        return div().into_any_element();
    }

    let closing = app.ui.player_closing;
    let has_song = app.player.read(cx).current().is_some();
    let viewport = window.viewport_size();
    let scroll = app.pane("player");
    let top = scroll.clone();
    let content = div()
        .flex()
        .flex_col()
        .flex_1()
        .min_h(theme::unit(0.0))
        .child(
            div()
                .id("player-scroll")
                .w_full()
                .flex_1()
                .min_h(theme::unit(0.0))
                .overflow_y_scroll()
                .track_scroll(&scroll)
                .child(
                    div()
                        .w_full()
                        .flex()
                        .flex_col()
                        .px(theme::unit(24.0))
                        .pt(theme::unit(16.0))
                        .pb(theme::unit(16.0))
                        .child(div().flex().justify_center().child(disc(
                            app,
                            cx,
                            theme::PLAYER_DISC,
                            has_song,
                        )))
                        .child(div().pt(theme::unit(16.0)).child(info(
                            app,
                            cx,
                            has_song,
                            viewport.width.as_f32(),
                        ))),
                ),
        );

    sheet::Sheet::new(
        "player",
        closing,
        |app, _, cx| {
            app.close_player();
            cx.notify();
        },
        content,
    )
    .mobile(true)
    .bar(true)
    .at_top(move || top.offset().y >= gpui::px(0.0))
    .show(window, cx)
}
