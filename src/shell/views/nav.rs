use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, App, Context, Div, InteractiveElement, IntoElement, ParentElement, Rgba, Stateful,
    StatefulInteractiveElement, Styled, Window, div,
};

use crate::shared::ui::views::design::icons;
use crate::shared::ui::views::design::theme;
use crate::shell::app::{Jondo, Route};

pub fn nav(app: &mut Jondo, cx: &mut Context<Jondo>, narrow: bool) -> AnyElement {
    if !narrow {
        return div().into_any_element();
    }

    let accent = app.accent;
    let room_color = if app.rooms.read(cx).live() {
        theme::GREEN
    } else if app.rooms.read(cx).active() {
        theme::AMBER
    } else {
        theme::MUTED
    };

    div()
        .w_full()
        .h(theme::unit(theme::NAV_HEIGHT))
        .flex_none()
        .flex()
        .bg(theme::BG)
        .child(nav_item(
            "nav-following",
            icons::USERS,
            app.route == Route::Following,
            accent,
            theme::MUTED,
            cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                app.open(Route::Following, cx);
                cx.notify();
            }),
        ))
        .child(nav_item(
            "nav-playlists",
            icons::LIST_MUSIC,
            matches!(app.route, Route::Home | Route::Playlist(_)),
            accent,
            theme::MUTED,
            cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                app.open(Route::Home, cx);
                cx.notify();
            }),
        ))
        .child(nav_item(
            "nav-rooms",
            icons::TURNTABLE,
            app.route == Route::Rooms,
            accent,
            room_color,
            cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                app.open(Route::Rooms, cx);
                cx.notify();
            }),
        ))
        .into_any_element()
}

fn nav_item(
    id: &'static str,
    icon: icons::Icon,
    active: bool,
    accent: Rgba,
    idle: Rgba,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    div()
        .id(id)
        .relative()
        .flex_1()
        .h_full()
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .on_click(on_click)
        .child(icons::icon(icon, 22.0, if active { accent } else { idle }))
        .when(active, |this| {
            this.child(
                div()
                    .absolute()
                    .left(theme::unit(0.0))
                    .right(theme::unit(0.0))
                    .bottom(theme::unit(0.0))
                    .h(theme::unit(2.0))
                    .bg(accent),
            )
        })
}
