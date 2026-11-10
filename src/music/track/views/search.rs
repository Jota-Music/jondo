use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, Context, Div, InteractiveElement, IntoElement, ParentElement, ScrollStrategy,
    Stateful, StatefulInteractiveElement, Styled, Window, div,
};
use std::rc::Rc;

use crate::music::library::views::playlist;
use crate::music::track::views::{list, row};
use crate::shared::i18n::t;
use crate::shared::music::domain::Channel;
use crate::shared::music::views::cover;
use crate::shared::ui::views::components::paint::{self, Scope};
use crate::shared::ui::views::components::{grid, hint, hover, loading, panel, text};
use crate::shared::ui::views::design::icons;
use crate::shared::ui::views::design::theme;
use crate::shell::app::{Jondo, Route};

pub fn show(app: &mut Jondo, window: &mut Window, cx: &mut Context<Jondo>) -> AnyElement {
    let query = app.search.query.clone();
    let videos =
        !app.search.songs.empty() || app.search.songs.loading || app.search.songs.error.is_some();
    let playlists = !app.search.playlists.empty()
        || app.search.playlists.loading
        || app.search.playlists.error.is_some();
    let channels = !app.search.channels.empty()
        || app.search.channels.loading
        || app.search.channels.error.is_some();
    let tabs = [
        videos.then_some(("videos", icons::CIRCLE_PLAY)),
        playlists.then_some(("playlists", icons::LIST_MUSIC)),
        channels.then_some(("channels", icons::USER_ROUND)),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();

    if !tabs.iter().any(|(id, _)| *id == app.search.tab) {
        app.search.tab = tabs
            .first()
            .map(|(id, _)| *id)
            .unwrap_or("videos")
            .to_owned();
    }

    let listable = matches!(app.search.tab.as_str(), "playlists" | "channels");
    let mut controls = div()
        .relative()
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .gap(theme::unit(8.0))
        .px(theme::unit(4.0))
        .pt(theme::unit(4.0))
        .pb(theme::unit(8.0));
    if !tabs.is_empty() {
        let mut bar = div()
            .flex()
            .flex_none()
            .gap(theme::unit(4.0))
            .p(theme::unit(4.0))
            .rounded(theme::unit(8.0))
            .bg(theme::SURFACE)
            .border_1()
            .border_color(theme::BORDER);
        for (id, icon) in &tabs {
            bar = bar.child(tab(app, cx, id, *icon));
        }

        controls = controls.child(bar);
    }

    if listable {
        controls = controls.child(grid::toggle(
            "search-view",
            !app.search.list,
            32.0,
            cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                app.search.list = false;
                cx.notify();
            }),
            cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                app.search.list = true;
                cx.notify();
            }),
        ));
    }

    controls = controls.child(
        div()
            .absolute()
            .left(theme::unit(0.0))
            .right(theme::unit(0.0))
            .top(theme::unit(0.0))
            .bottom(theme::unit(0.0))
            .flex()
            .items_center()
            .justify_center()
            .px(theme::unit(120.0))
            .child(text::truncated(
                query,
                16.0,
                theme::TEXT,
                theme::Weight::SemiBold,
            )),
    );

    let body = match app.search.tab.as_str() {
        "playlists" => playlists_content(app, window, cx),
        "channels" => channels_content(app, window, cx),
        _ => videos_content(app, cx),
    };

    div()
        .size_full()
        .flex()
        .flex_col()
        .child(panel(
            div()
                .size_full()
                .flex()
                .flex_col()
                .child(controls)
                .child(body),
        ))
        .into_any_element()
}

fn tab(app: &mut Jondo, cx: &mut Context<Jondo>, id: &str, icon: icons::Icon) -> Stateful<Div> {
    let active = app.search.tab == id;
    let element = gpui::ElementId::Name(format!("search-tab-{id}").into());
    let fade = hover::value(&element, std::time::Instant::now());
    let id = id.to_owned();
    div()
        .id(element.clone())
        .w(theme::unit(32.0))
        .h(theme::unit(32.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(theme::unit(6.0))
        .cursor_pointer()
        .when(active, |this| this.bg(theme::ZINC_800))
        .on_hover({
            let element = element.clone();
            move |on: &bool, window, cx| {
                hover::set_in(
                    Scope::Page,
                    &element,
                    *on,
                    std::time::Instant::now(),
                    window.last_input_was_touch(),
                );
                paint::paint(Scope::Page, cx);
            }
        })
        .on_click(cx.listener(move |app, _: &gpui::ClickEvent, _, cx| {
            app.search.tab = id.clone();
            app.clear_selection();
            cx.notify();
        }))
        .child(icons::icon(
            icon,
            16.0,
            if active {
                theme::rgb(0xffffff)
            } else {
                theme::mix(theme::FAINT, theme::ZINC_300, fade)
            },
        ))
}

fn videos_content(app: &mut Jondo, cx: &mut Context<Jondo>) -> AnyElement {
    if app.search.songs.loading {
        return loading::spinner().into_any_element();
    }

    if let Some(error) = &app.search.songs.error {
        return hint::message(text::text(error.clone(), 13.0, theme::DANGER)).into_any_element();
    }

    if app.search.songs.empty() {
        return hint::message(text::text(t("No results"), 13.0, theme::MUTED)).into_any_element();
    }

    let table = app.search_rows();
    let scroll = app.scroll("search");
    if app.search.scroll_top {
        scroll.scroll_to_item(0, ScrollStrategy::Top);
        app.search.scroll_top = false;
    }

    list::show(
        app,
        cx,
        list::List {
            key: "search".to_owned(),
            id: "youtube-results",
            table: table.clone(),
            playlist: None,
            removable: false,
            shape: row::Shape::Video,
            focus: false,
        },
    )
}

fn playlists_content(app: &mut Jondo, window: &mut Window, cx: &mut Context<Jondo>) -> AnyElement {
    if app.search.playlists.loading {
        return loading::spinner().into_any_element();
    }

    if let Some(error) = &app.search.playlists.error {
        return hint::message(text::text(error.clone(), 13.0, theme::DANGER)).into_any_element();
    }

    if app.search.playlists.empty() {
        return hint::message(text::text(t("No playlists found."), 13.0, theme::MUTED))
            .into_any_element();
    }

    let playlists = app.search.playlists.list().to_vec();
    playlist::results(
        app,
        window,
        cx,
        "search-playlists",
        "search-playlists",
        Rc::new(playlists),
        !app.search.list,
    )
}

fn channels_content(app: &mut Jondo, window: &mut Window, cx: &mut Context<Jondo>) -> AnyElement {
    if app.search.channels.loading {
        return loading::spinner().into_any_element();
    }

    if let Some(error) = &app.search.channels.error {
        return hint::message(text::text(error.clone(), 13.0, theme::DANGER)).into_any_element();
    }

    if app.search.channels.empty() {
        return hint::message(text::text(t("No channels found."), 13.0, theme::MUTED))
            .into_any_element();
    }

    let channels = app.search.channels.list().to_vec();
    let scroll = app.pane("search-channels");
    if app.search.list {
        let rows = channels
            .iter()
            .map(|channel| channel_row(app, cx, channel).into_any_element())
            .collect();
        grid::list("search-channels", &scroll, rows)
    } else {
        let cell = grid::cell(window);
        let tiles = channels
            .iter()
            .map(|channel| channel_tile(app, cx, channel, cell).into_any_element())
            .collect();
        grid::show("search-channels", &scroll, window, tiles)
    }
}

fn channel_tile(
    app: &mut Jondo,
    cx: &mut Context<Jondo>,
    channel: &Channel,
    cell: f32,
) -> AnyElement {
    let id = channel.id.clone();
    div()
        .id(format!("search-channel-{}", channel.id))
        .w(theme::unit(cell))
        .flex_none()
        .flex()
        .flex_col()
        .cursor_pointer()
        .on_click(cx.listener(move |app, _: &gpui::ClickEvent, _, cx| {
            app.open(Route::Channel(id.clone()), cx);
            cx.notify();
        }))
        .child(
            div()
                .px(theme::unit(6.0))
                .pt(theme::unit(6.0))
                .child(cover::art(
                    &mut app.covers,
                    channel.avatar.as_deref().unwrap_or_default(),
                    cell - 12.0,
                    cell - 12.0,
                    6.0,
                )),
        )
        .child(
            div()
                .px(theme::unit(6.0))
                .pt(theme::unit(8.0))
                .pb(theme::unit(6.0))
                .child(text::truncated(
                    channel.name.clone(),
                    14.0,
                    theme::TEXT,
                    theme::Weight::Medium,
                )),
        )
        .into_any_element()
}

fn channel_row(app: &mut Jondo, cx: &mut Context<Jondo>, channel: &Channel) -> AnyElement {
    let route = Route::Channel(channel.id.clone());
    div()
        .id(format!("search-channel-row-{}", channel.id))
        .w_full()
        .h(theme::unit(52.0))
        .flex()
        .items_center()
        .rounded(theme::unit(6.0))
        .cursor_pointer()
        .hover(|style| style.bg(theme::ZINC_900))
        .on_click(cx.listener(move |app, _: &gpui::ClickEvent, _, cx| {
            app.open(route.clone(), cx);
            cx.notify();
        }))
        .child(div().w(theme::unit(8.0)))
        .child(cover::art(
            &mut app.covers,
            channel.avatar.as_deref().unwrap_or_default(),
            40.0,
            40.0,
            6.0,
        ))
        .child(div().w(theme::unit(12.0)))
        .child(
            div()
                .flex_1()
                .min_w(theme::unit(0.0))
                .flex()
                .flex_col()
                .gap(theme::unit(2.0))
                .child(text::truncated(
                    channel.name.clone(),
                    14.0,
                    theme::TEXT,
                    theme::Weight::Regular,
                ))
                .child(text::truncated(
                    "YouTube",
                    12.0,
                    theme::FAINT,
                    theme::Weight::Regular,
                )),
        )
        .child(div().w(theme::unit(8.0)))
        .into_any_element()
}
