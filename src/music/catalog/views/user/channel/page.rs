use crate::music::track::views::{actions, list, row};
use crate::shared::i18n::t;
use crate::shared::music::domain::Follow;
use crate::shared::ui::views::components::{grid, hint, loading};
use crate::shared::ui::views::design::icons;
use crate::shared::ui::views::design::theme;
use crate::shell::app::{Jondo, Route};
use crate::shell::views::menu;
use crate::shell::views::page;
use crate::shell::views::toolbar::{self, Toolbar};
use gpui::prelude::FluentBuilder as _;
use gpui::{AnyElement, Context, Div, IntoElement, ParentElement, Styled, Window, div};

use super::playlists::playlists;
use super::tabs::tab_button;

fn bar(app: &mut Jondo, window: &mut Window, cx: &mut Context<Jondo>) -> Div {
    let id = match app.route.clone() {
        Route::Channel(id) => id,
        _ => String::new(),
    };

    let query = app.ui.channel_query.clone();
    let refreshing = app.catalog.read(cx).channels.loading(&id);
    toolbar::show(
        app,
        window,
        cx,
        Toolbar {
            id: "channel",
            field: "channel-search",
            placeholder: t("Search in channel"),
            query,
            refreshing,
            origin: "channel-order",
            order: app.ui.channel_order.label().to_owned(),
            items: menu::channel_order(app.ui.channel_order),
            clear: Box::new(cx.listener(|app, _: &gpui::ClickEvent, window, cx| {
                app.ui.channel_query.clear();
                app.set_field("channel-search", "", window, cx);
                cx.notify();
            })),
            reload: Box::new(cx.listener(move |app, _: &gpui::ClickEvent, _, cx| {
                app.reload_channel(&id, cx);
                cx.notify();
            })),
        },
    )
}

/// The channel before its data lands: the avatar and the name lines shimmer in
/// the header, with the follow slot already where it will stay, and the tab
/// content keeps its frame.
fn skeleton(app: &mut Jondo, window: &mut Window, cx: &mut Context<Jondo>) -> AnyElement {
    let header = page::pending(64.0, 32.0).child(
        div()
            .w(theme::unit(32.0))
            .h(theme::unit(32.0))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .rounded(theme::unit(6.0))
            .border_1()
            .border_color(theme::ZINC_800)
            .bg(theme::SURFACE)
            .child(icons::icon(icons::HEART, 14.0, theme::ZINC_700)),
    );

    let tab = app.ui.channel_tab.clone();
    let grid = app.ui.channel_grid;
    let tabs = div()
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .flex()
                .flex_none()
                .gap(theme::unit(4.0))
                .p(theme::unit(4.0))
                .rounded(theme::unit(8.0))
                .border_1()
                .border_color(theme::BORDER)
                .bg(theme::SURFACE)
                .child(tab_button(
                    cx,
                    "channel-tab-videos",
                    icons::CIRCLE_PLAY,
                    tab == "videos",
                    "videos",
                ))
                .child(tab_button(
                    cx,
                    "channel-tab-playlists",
                    icons::LIST_MUSIC,
                    tab == "playlists",
                    "playlists",
                )),
        )
        .when(tab == "playlists", |this| {
            this.child(grid::toggle(
                "channel-view",
                grid,
                32.0,
                cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                    app.ui.channel_grid = true;
                    cx.notify();
                }),
                cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                    app.ui.channel_grid = false;
                    cx.notify();
                }),
            ))
        });

    let content = if tab == "playlists" {
        page::panel(loading::spinner())
    } else {
        div()
            .flex_1()
            .min_h(theme::unit(0.0))
            .flex()
            .flex_col()
            .gap(theme::unit(8.0))
            .child(bar(app, window, cx))
            .child(loading::rows())
            .into_any_element()
    };

    div()
        .size_full()
        .flex()
        .flex_col()
        .gap(theme::unit(16.0))
        .child(header)
        .child(
            div()
                .flex_1()
                .min_h(theme::unit(0.0))
                .flex()
                .flex_col()
                .gap(theme::unit(8.0))
                .child(tabs)
                .child(content),
        )
        .into_any_element()
}

pub fn show(app: &mut Jondo, window: &mut Window, cx: &mut Context<Jondo>) -> AnyElement {
    let Route::Channel(id) = app.route.clone() else {
        return div().into_any_element();
    };

    let Some(channel) = app.channel(&id, cx) else {
        if app.catalog.read(cx).channels.loading(&id) {
            return skeleton(app, window, cx);
        }

        return hint::plain(t("Channel not found")).into_any_element();
    };

    let channel_id = channel.id.clone();
    let channel_name = channel.name.clone();
    let channel_avatar = channel.avatar.clone();
    let channel_playlists = channel.playlists.clone();

    let follow = Follow::youtube(&channel_id, &channel_name, channel_avatar.clone());
    let summary = follow.clone();
    let share = channel_id.clone();
    let header = page::show(
        app,
        cx,
        page::Header {
            cover: channel_avatar.as_deref(),
            covers: &[],
            radius: 32.0,
            title: &channel_name,
            subtitle: Some("YouTube".to_owned()),
            link: None,
            save: None,
            follow: Some(follow),
            play: None,
            extra: None,
            actions: Some(Box::new(move |app: &Jondo, _: &gpui::App| {
                actions::channel(app, &summary, &share)
            })),
        },
    );

    let toolbar = bar(app, window, cx);

    let table = app.channel_rows(&id, cx);
    let videos = page::panel(list::show(
        app,
        cx,
        list::List {
            key: format!("channel:{id}"),
            id: "channel-videos",
            table: table.clone(),
            playlist: None,
            removable: false,
            shape: row::Shape::Song,
            focus: true,
        },
    ));

    let tab = app.ui.channel_tab.clone();
    let grid = app.ui.channel_grid;
    let tabs = div()
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .flex()
                .flex_none()
                .gap(theme::unit(4.0))
                .p(theme::unit(4.0))
                .rounded(theme::unit(8.0))
                .border_1()
                .border_color(theme::BORDER)
                .bg(theme::SURFACE)
                .child(tab_button(
                    cx,
                    "channel-tab-videos",
                    icons::CIRCLE_PLAY,
                    tab == "videos",
                    "videos",
                ))
                .child(tab_button(
                    cx,
                    "channel-tab-playlists",
                    icons::LIST_MUSIC,
                    tab == "playlists",
                    "playlists",
                )),
        )
        .when(tab == "playlists", |this| {
            this.child(grid::toggle(
                "channel-view",
                grid,
                32.0,
                cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                    app.ui.channel_grid = true;
                    cx.notify();
                }),
                cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                    app.ui.channel_grid = false;
                    cx.notify();
                }),
            ))
        });

    let content = if tab == "playlists" {
        playlists(app, window, cx, &channel_id, &channel_playlists)
    } else {
        div()
            .flex_1()
            .min_h(theme::unit(0.0))
            .flex()
            .flex_col()
            .gap(theme::unit(8.0))
            .child(toolbar)
            .child(videos)
            .into_any_element()
    };

    div()
        .size_full()
        .flex()
        .flex_col()
        .gap(theme::unit(16.0))
        .child(header)
        .child(
            div()
                .flex_1()
                .min_h(theme::unit(0.0))
                .flex()
                .flex_col()
                .gap(theme::unit(8.0))
                .child(tabs)
                .child(content),
        )
        .into_any_element()
}
