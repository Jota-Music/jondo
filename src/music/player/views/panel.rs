use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement, Styled, Window, div,
};

use crate::music::track::views::actions;
use crate::shared::ui::views::design::theme;
use crate::shell::app::Jondo;
use crate::shell::views::menu::{Command, Menu};

use super::{disc, info};

pub fn panel(app: &mut Jondo, window: &mut Window, cx: &mut Context<Jondo>) -> AnyElement {
    let narrow = window.viewport_size().width < theme::unit(768.0);
    if narrow || app.ui.compact || app.player.read(cx).current().is_none() {
        return div().into_any_element();
    }

    let has_song = app.player.read(cx).current().is_some();
    div()
        .w_full()
        .flex_none()
        .bg(theme::BG)
        .pt(theme::unit(24.0))
        .pb(theme::unit(24.0))
        .on_mouse_down(
            gpui::MouseButton::Right,
            cx.listener(|app, event: &gpui::MouseDownEvent, _, cx| {
                let items = app
                    .player
                    .read(cx)
                    .current()
                    .cloned()
                    .map(|song| {
                        actions::track(
                            app,
                            cx,
                            Vec::new(),
                            vec![song],
                            Command::Toggle,
                            None,
                            None,
                            None,
                            Vec::new(),
                        )
                    })
                    .unwrap_or_default();
                if items.is_empty() {
                    return;
                }

                app.ui.menu = Some(Menu::context(event.position, None, items, 0.0));
                cx.notify();
            }),
        )
        .child(crate::shared::ui::views::components::column(
            div()
                .flex()
                .gap(theme::unit(24.0))
                .child(disc::disc(app, cx, theme::PLAYER_DISC, has_song))
                .child(
                    div()
                        .flex_1()
                        .min_w(theme::unit(0.0))
                        .flex()
                        .flex_col()
                        .child(info::info(
                            app,
                            cx,
                            has_song,
                            window.viewport_size().width.as_f32(),
                        )),
                ),
        ))
        .into_any_element()
}
