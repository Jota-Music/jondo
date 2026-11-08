use gpui::prelude::FluentBuilder as _;
use gpui::{
    App, Context, Div, InteractiveElement, IntoElement, ParentElement, Stateful,
    StatefulInteractiveElement, Styled, Window, div,
};

use crate::shared::i18n::t;
use crate::shared::music::domain::Source;
use crate::shared::music::views::cover::brand_mono;
use crate::shared::ui::views::components::{grid as layout, hover, text};
use crate::shared::ui::views::design::icons;
use crate::shared::ui::views::design::theme;
use crate::shell::app::{Filter, Jondo};

use super::{Scope, filter, grid, set_filter, set_grid};

pub(super) fn controls(
    app: &mut Jondo,
    cx: &mut Context<Jondo>,
    narrow: bool,
    scope: Scope,
) -> Div {
    let height = if narrow { 40.0 } else { 32.0 };
    let filter = filter(app, scope);
    let grid = grid(app, scope);
    let mut sources = div()
        .flex()
        .rounded(theme::unit(6.0))
        .border_1()
        .border_color(theme::BORDER)
        .child(segment_text(
            "filter-all",
            t("All"),
            filter == Filter::All,
            height,
            cx.listener(move |app, _: &gpui::ClickEvent, _, cx| {
                set_filter(app, scope, Filter::All);
                cx.notify();
            }),
        ))
        .child(segment(
            "filter-youtube",
            icons::Icon::YouTube,
            if narrow { 20.0 } else { 16.0 },
            filter == Filter::YouTube,
            true,
            height,
            cx.listener(move |app, _: &gpui::ClickEvent, _, cx| {
                set_filter(app, scope, Filter::YouTube);
                cx.notify();
            }),
        ))
        .child(segment(
            "filter-spotify",
            icons::Icon::Spotify,
            if narrow { 18.0 } else { 14.0 },
            filter == Filter::Spotify,
            true,
            height,
            cx.listener(move |app, _: &gpui::ClickEvent, _, cx| {
                set_filter(app, scope, Filter::Spotify);
                cx.notify();
            }),
        ));
    if scope == Scope::Library {
        sources = sources.child(segment(
            "filter-local",
            icons::LIBRARY,
            if narrow { 18.0 } else { 14.0 },
            filter == Filter::Local,
            true,
            height,
            cx.listener(move |app, _: &gpui::ClickEvent, _, cx| {
                set_filter(app, scope, Filter::Local);
                cx.notify();
            }),
        ));
    }

    div()
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .px(theme::unit(4.0))
        .pt(theme::unit(4.0))
        .pb(theme::unit(8.0))
        .child(
            div()
                .flex()
                .items_center()
                .gap(theme::unit(8.0))
                .child(sources)
                .when(scope == Scope::Library && filter == Filter::Local, |row| {
                    row.child(
                        div()
                            .id("new-playlist")
                            .debug_selector(|| "new-playlist".to_owned())
                            .w(theme::unit(height))
                            .h(theme::unit(height))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(theme::unit(6.0))
                            .border_1()
                            .border_color(theme::BORDER)
                            .bg(theme::SURFACE)
                            .cursor_pointer()
                            .hover(|style| style.bg(theme::ZINC_900))
                            .on_click(cx.listener(|app, _: &gpui::ClickEvent, window, cx| {
                                app.ui.create_open = true;
                                app.set_field("create", "", window, cx);
                                cx.notify();
                            }))
                            .child(icons::icon(
                                icons::PLUS,
                                if narrow { 20.0 } else { 16.0 },
                                theme::MUTED,
                            )),
                    )
                }),
        )
        .child(layout::toggle(
            "shelf-view",
            grid,
            height,
            cx.listener(move |app, _: &gpui::ClickEvent, _, cx| {
                set_grid(app, scope, true);
                cx.notify();
            }),
            cx.listener(move |app, _: &gpui::ClickEvent, _, cx| {
                set_grid(app, scope, false);
                cx.notify();
            }),
        ))
}

pub(super) fn segment_text(
    id: &'static str,
    label: &'static str,
    active: bool,
    height: f32,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let element: gpui::ElementId = id.into();
    let fade = hover::value(&element, std::time::Instant::now());
    div()
        .id(element.clone())
        .debug_selector(move || id.to_owned())
        .h(theme::unit(height))
        .px(theme::unit(12.0))
        .flex()
        .items_center()
        .justify_center()
        .when(active, |this| this.bg(theme::ZINC_800))
        .cursor_pointer()
        .on_hover(move |on: &bool, _, cx| {
            hover::set(&element, *on, std::time::Instant::now());
            cx.refresh_windows();
        })
        .on_click(on_click)
        .child(text::weighted(
            label,
            12.0,
            if active {
                theme::rgb(0xffffff)
            } else {
                theme::mix(theme::FAINT, theme::rgb(0xffffff), fade)
            },
            theme::Weight::Medium,
        ))
}

pub(super) fn segment(
    id: &'static str,
    icon: icons::Icon,
    size: f32,
    active: bool,
    divider: bool,
    height: f32,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let is_brand = matches!(icon, icons::Icon::YouTube | icons::Icon::Spotify);
    let element: gpui::ElementId = id.into();
    let fade = hover::value(&element, std::time::Instant::now());
    let color = if active {
        theme::rgb(0xffffff)
    } else {
        theme::mix(theme::FAINT, theme::rgb(0xffffff), fade)
    };

    div()
        .id(element.clone())
        .debug_selector(move || id.to_owned())
        .relative()
        .w(theme::unit(if height > 32.0 { 40.0 } else { 32.0 }))
        .h(theme::unit(height))
        .flex()
        .items_center()
        .justify_center()
        .when(active, |this| this.bg(theme::ZINC_800))
        .when(divider, |this| {
            this.child(
                div()
                    .absolute()
                    .left(theme::unit(0.0))
                    .top(theme::unit(0.0))
                    .bottom(theme::unit(0.0))
                    .w(theme::unit(1.0))
                    .bg(theme::BORDER),
            )
        })
        .cursor_pointer()
        .on_hover(move |on: &bool, _, cx| {
            hover::set(&element, *on, std::time::Instant::now());
            cx.refresh_windows();
        })
        .on_click(on_click)
        .child(if is_brand {
            brand_mono(
                if icon == icons::Icon::YouTube {
                    Source::YouTube
                } else {
                    Source::Spotify
                },
                size,
                color,
            )
        } else {
            icons::icon(icon, size, color).into_any_element()
        })
}
