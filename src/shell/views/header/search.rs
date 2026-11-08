use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, Context, Div, Focusable, InteractiveElement, IntoElement, ParentElement, Stateful,
    StatefulInteractiveElement, Styled, Window, div,
};

use gpui_kit::component::input::InputState;

use crate::shared::i18n::t;
use crate::shared::music::domain::Source;
use crate::shared::music::views::cover::{brand, brand_mono};
use crate::shared::ui::views::components::field;
use crate::shared::ui::views::design::icons;
use crate::shared::ui::views::design::theme;
use crate::shell::app::{Jondo, SearchType, SpotifyState};
use crate::shell::views::menu::{self, Menu};

use super::{FIELD_HEIGHT, GAP, MOBILE_FIELD_HEIGHT, MOBILE_GAP, MOBILE_PADDING};

pub(super) fn search_panel(
    app: &mut Jondo,
    window: &mut Window,
    cx: &mut Context<Jondo>,
    narrow: bool,
    icon: f32,
) -> AnyElement {
    let field = app.field("search", "", &placeholder(app), window, cx);
    if app.ui.search_focus {
        field.update(cx, |state, cx| state.focus_handle(cx).focus(window, cx));
        app.ui.search_focus = false;
    }

    let field_height = if narrow {
        MOBILE_FIELD_HEIGHT
    } else {
        FIELD_HEIGHT
    };

    let gap = if narrow { MOBILE_GAP } else { GAP };

    let toggle = source_toggle(app, cx, narrow, field_height, icon);
    let submit = submit_button(app, cx, field_height, icon);
    let row = div()
        .flex()
        .items_center()
        .gap(theme::unit(gap))
        .child(pill(app, cx, &field, field_height, icon, narrow))
        .child(submit);
    let toggle_row = div()
        .h(theme::unit(field_height))
        .rounded(theme::unit(8.0))
        .bg(theme::SURFACE)
        .border_1()
        .border_color(theme::BORDER)
        .overflow_hidden()
        .when(narrow, |row| row.self_start())
        .child(toggle);

    let inner = div()
        .flex()
        .flex_col()
        .gap(theme::unit(gap))
        .when(narrow, |inner| inner.child(toggle_row))
        .child(row);

    div()
        .w_full()
        .flex()
        .justify_center()
        .py(theme::unit(8.0))
        .child(
            div()
                .w(theme::unit(theme::CONTENT_WIDTH))
                .max_w_full()
                .min_w_0()
                .when(narrow, |inner| inner.px(theme::unit(MOBILE_PADDING)))
                .child(inner),
        )
        .into_any_element()
}

fn source_toggle(
    app: &mut Jondo,
    cx: &mut Context<Jondo>,
    narrow: bool,
    height: f32,
    icon: f32,
) -> Div {
    let spotify = app.store.spotify.is_some();
    let ready = app.settings.read(cx).spotify == SpotifyState::On;
    let live = app.live_source();
    let width = if narrow { 44.0 } else { 40.0 };
    let divider = div().w(theme::unit(1.0)).h_full().bg(theme::ZINC_800);

    let mut group = div()
        .flex()
        .items_center()
        .h(theme::unit(height))
        .child(
            div()
                .id("source-youtube")
                .debug_selector(|| "source-youtube".to_owned())
                .w(theme::unit(width))
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .rounded_l(theme::unit(8.0))
                .cursor_pointer()
                .when(live == Source::YouTube, |this| this.bg(theme::ZINC_800))
                .on_click(cx.listener(|app, _: &gpui::ClickEvent, window, cx| {
                    app.ui.search_source = Source::YouTube;
                    app.ui.search_focus = true;
                    app.refresh_search_placeholder(window, cx);
                    cx.notify();
                }))
                .child(if live == Source::YouTube {
                    brand(Source::YouTube, icon)
                } else {
                    brand_mono(Source::YouTube, icon, theme::FAINT)
                }),
        )
        .when(spotify, |this| {
            this.child(
                div()
                    .id("source-spotify")
                    .w(theme::unit(width))
                    .h_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .when(live == Source::Spotify, |this| this.bg(theme::ZINC_800))
                    .on_click(cx.listener(|app, _: &gpui::ClickEvent, window, cx| {
                        app.ui.search_source = Source::Spotify;
                        app.ui.search_focus = true;
                        app.refresh_search_placeholder(window, cx);
                        cx.notify();
                    }))
                    .child(if live == Source::Spotify && ready {
                        brand(Source::Spotify, icon)
                    } else {
                        brand_mono(Source::Spotify, icon, theme::FAINT)
                    }),
            )
        })
        .child(divider);
    if live == Source::Spotify {
        group = group.child(type_select(app, cx, narrow, height));
    }

    group
}

fn type_select(
    app: &mut Jondo,
    cx: &mut Context<Jondo>,
    narrow: bool,
    height: f32,
) -> Stateful<Div> {
    let width = 116.0;
    let open = app
        .ui
        .menu
        .as_ref()
        .is_some_and(|menu| menu.origin == Some("search-type"));
    div()
        .id("search-type")
        .debug_selector(|| "search-type".to_owned())
        .w(theme::unit(width))
        .h_full()
        .relative()
        .flex()
        .items_center()
        .cursor_pointer()
        .when(open, |this| this.bg(theme::ZINC_900))
        .child(
            div()
                .flex_1()
                .h_full()
                .flex()
                .items_center()
                .justify_between()
                .pl(theme::unit(if narrow { 8.0 } else { 12.0 }))
                .pr(theme::unit(if narrow { 4.0 } else { 8.0 }))
                .child(crate::shared::ui::views::components::text::text(
                    app.ui.search_type.label().to_owned(),
                    14.0,
                    theme::TEXT,
                ))
                .child(icons::icon(icons::CHEVRON_DOWN, 12.0, theme::MUTED)),
        )
        .child(div().w(theme::unit(1.0)).h_full().bg(theme::ZINC_800))
        .role(gpui::Role::Button)
        .aria_expanded(open)
        .aria_label(format!(
            "{}: {}",
            t("Search type"),
            app.ui.search_type.label()
        ))
        .on_click(cx.listener(|app, event: &gpui::ClickEvent, _, cx| {
            app.ui.menu = Some(
                Menu::choice(
                    event.position(),
                    t("Search type"),
                    menu::search_types(app.ui.search_type),
                )
                .origin("search-type"),
            );
            cx.notify();
        }))
        .when(height > 0.0, |this| this)
}

fn pill(
    app: &mut Jondo,
    cx: &mut Context<Jondo>,
    field: &gpui::Entity<InputState>,
    height: f32,
    icon: f32,
    narrow: bool,
) -> Div {
    let padding = if narrow { 12.0 } else { 16.0 };
    let query = app.ui.query.clone();
    let mut pill = div()
        .flex_1()
        .min_w_0()
        .h(theme::unit(height))
        .flex()
        .items_center()
        .rounded(theme::unit(8.0))
        .bg(theme::SURFACE)
        .border_1()
        .border_color(theme::BORDER)
        .when(!narrow, |this| {
            this.child(source_toggle(app, cx, narrow, height, icon))
        })
        .child(
            div()
                .flex_1()
                .min_w_0()
                .h_full()
                .flex()
                .items_center()
                .px(theme::unit(padding))
                .child(field::input(field)),
        );
    if !query.is_empty() {
        pill = pill.child(
            div()
                .id("search-clear")
                .w(theme::unit(24.0))
                .h(theme::unit(24.0))
                .mr(theme::unit(8.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded(theme::unit(4.0))
                .cursor_pointer()
                .hover(|style| style.bg(theme::ZINC_800))
                .on_click(cx.listener(|app, _: &gpui::ClickEvent, window, cx| {
                    app.ui.query.clear();
                    if let Some(field) = app.fields.get("search").cloned() {
                        field.update(cx, |state, cx| state.set_value("", window, cx));
                    }

                    cx.notify();
                }))
                .child(icons::icon(icons::X, 14.0, theme::FAINT)),
        );
    }

    pill
}

fn submit_button(
    app: &mut Jondo,
    cx: &mut Context<Jondo>,
    height: f32,
    icon: f32,
) -> Stateful<Div> {
    let fill = app.accent;
    let color = app.binary;
    div()
        .id("search-submit")
        .w(theme::unit(height))
        .h(theme::unit(height))
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .rounded(theme::unit(8.0))
        .bg(fill)
        .cursor_pointer()
        .hover(move |style| style.bg(theme::mix(fill, theme::SURFACE, 0.25)))
        .on_click(cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
            app.submit_search(cx);
            cx.notify();
        }))
        .child(icons::icon(icons::SEARCH_BOLD, icon, color))
}

pub(crate) fn placeholder(app: &Jondo) -> String {
    if app.live_source() == Source::YouTube {
        return t("Search in YouTube (videos, playlists or channels)").to_owned();
    }

    match app.ui.search_type {
        SearchType::Track => t("Song, artist, or Spotify link/ID...").to_owned(),
        SearchType::User => t("Spotify username or link...").to_owned(),
        kind => t("Paste a Spotify {} link, URI or ID...").replace("{}", kind.label()),
    }
}
