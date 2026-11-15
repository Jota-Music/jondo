use gpui::prelude::FluentBuilder as _;
use gpui::{
    App, Context, Div, InteractiveElement, IntoElement, ParentElement, Stateful,
    StatefulInteractiveElement, Styled, Window, div,
};

use crate::shared::i18n::t;
use crate::shared::music::domain::Source;
use crate::shared::music::views::cover::brand_mono;
use crate::shared::ui::views::components::paint::{self, Scope as PaintScope};
use crate::shared::ui::views::components::{grid as layout, hover};
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
        .bg(theme::SURFACE)
        .child(
            segment(
                "filter-all",
                icons::Icon::Asterisk,
                if narrow { 20.0 } else { 16.0 },
                filter == Filter::All,
                false,
                height,
                cx.listener(move |app, _: &gpui::ClickEvent, _, cx| {
                    set_filter(app, scope, Filter::All);
                    cx.notify();
                }),
            )
            .aria_label(t("All")),
        )
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
        // On a phone the bar takes the same 12 the page heads carry, so the
        // filters line up with the title instead of running wider than it.
        .px(theme::unit(if narrow { 12.0 } else { 4.0 }))
        .pt(theme::unit(if narrow { 8.0 } else { 4.0 }))
        .pb(theme::unit(8.0))
        .child(
            div()
                .flex()
                .items_center()
                .gap(theme::unit(8.0))
                .child(sources)
                .when(scope == Scope::Library && filter == Filter::Local, |row| {
                    row.child(hover::hover_in(
                        PaintScope::Page,
                        "new-playlist",
                        true,
                        move |fade| {
                            div()
                                .w(theme::unit(height))
                                .h(theme::unit(height))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(theme::unit(6.0))
                                .child(icons::icon(
                                    icons::PLUS,
                                    18.0,
                                    theme::mix(theme::MUTED, theme::TEXT, fade),
                                ))
                        },
                        cx.listener(|app, _: &gpui::ClickEvent, window, cx| {
                            app.ui.create_open = true;
                            app.set_field("create", "", window, cx);
                            cx.notify();
                        }),
                    ))
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
    let element = id.into();
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
        .on_hover(move |on: &bool, window, cx| {
            hover::set_in(
                PaintScope::Page,
                &element,
                *on,
                std::time::Instant::now(),
                window.last_input_was_touch(),
            );
            paint::paint(PaintScope::Page, cx);
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
