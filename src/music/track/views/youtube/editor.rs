use std::time::Instant;

use gpui::prelude::FluentBuilder as _;
use gpui::{
    Context, Focusable as _, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Window, div,
};

use gpui_kit::component::tooltip::Tooltip;

use crate::shared::i18n::t;
use crate::shared::ui::views::components::paint::Scope;
use crate::shared::ui::views::components::{button, field, hover, text};
use crate::shared::ui::views::design::icons::{self, Icon};
use crate::shared::ui::views::design::theme;
use crate::shell::app::Jondo;
use crate::shell::views::sheet;

use super::actions::{changed, save, switch};
use super::preview::preview_bar;
use super::results::results;
use super::{Mode, submit};

pub fn show(app: &mut Jondo, window: &mut Window, cx: &mut Context<Jondo>) -> impl IntoElement {
    if !app.editor.open {
        return div().into_any_element();
    }

    let closing = app.ui.closing;
    let assign = app.editor.mode == Mode::Assign;
    let song_name = app
        .editor
        .song
        .as_ref()
        .map(|song| song.name.clone())
        .unwrap_or_default();
    let link_value = app.editor.link.clone();
    let search_value = app.editor.search.clone();
    let link_field = app.field("youtube-link", &link_value, t("ID or URL"), window, cx);
    let search_field = app.field(
        "youtube-search",
        &search_value,
        t("Song, artist or link..."),
        window,
        cx,
    );

    if app.editor.focus {
        if assign {
            app.set_field("youtube-link", &link_value, window, cx);
            link_field.update(cx, |state, cx| {
                state.focus_handle(cx).focus(window, cx);
                state.select_all(window, cx);
            });
        } else {
            app.set_field("youtube-search", &search_value, window, cx);
            search_field.update(cx, |state, cx| {
                state.focus_handle(cx).focus(window, cx);
                state.select_all(window, cx);
            });
        }

        app.editor.focus = false;
    }

    let field = if assign { &link_field } else { &search_field };
    let focused = field.read(cx).focus_handle(cx).is_focused(window);
    let value = if assign {
        link_value.clone()
    } else {
        search_value.clone()
    };

    let hint = if assign {
        t("Paste the video ID or URL.")
    } else {
        t("Search YouTube and pick the right video.")
    };

    let slide = if cx.reduce_motion() {
        if assign { 0.0 } else { 1.0 }
    } else {
        app.editor.slide.value(Instant::now())
    };

    let mut row = div()
        .flex_none()
        .h(theme::unit(40.0))
        .flex()
        .items_center()
        .rounded(theme::unit(6.0))
        .border_1()
        .border_color(if focused {
            theme::ZINC_600
        } else {
            theme::BORDER
        })
        .bg(theme::SURFACE)
        .overflow_hidden()
        .block_mouse_except_scroll()
        .child(
            div()
                .h_full()
                .flex()
                .flex_none()
                .child(tab(cx, Mode::Assign, 1.0 - slide, icons::LINK_2))
                .child(tab(cx, Mode::Search, slide, icons::SEARCH_BOLD)),
        );
    if assign && app.editor.resolving {
        row = row.child(
            div()
                .ml(theme::unit(12.0))
                .flex_none()
                .child(button::spinner(16.0, theme::white(0.3))),
        );
    }

    row = row.child(
        div()
            .flex_1()
            .min_w(theme::unit(0.0))
            .h_full()
            .flex()
            .items_center()
            .px(theme::unit(12.0))
            .child(field::input(field)),
    );
    if !value.is_empty() {
        row = row.child(clear(cx, assign));
    }

    let body_scroll = app.pane("youtube-body");
    let top = body_scroll.clone();
    let body = div()
        .id("youtube-body")
        .flex_1()
        .min_h(theme::unit(0.0))
        .overflow_y_scroll()
        .track_scroll(&body_scroll)
        .child(
            div()
                .p(theme::unit(20.0))
                .flex()
                .flex_col()
                .gap(theme::unit(12.0))
                .child(text::text(hint, 12.0, theme::FAINT))
                .child(row)
                .when(slide > 0.0, |this| {
                    this.child(results(app, window, cx, &link_value, slide))
                }),
        );

    let mut footer = div().flex_none().border_t_1().border_color(theme::BORDER);
    if app.preview.video.is_some() {
        footer = footer.child(preview_bar(app, cx));
    }

    footer = footer.child(
        div()
            .flex()
            .justify_end()
            .p(theme::unit(12.0))
            .child(save_button(app, cx, changed(&app.editor))),
    );

    let panel = div()
        .relative()
        .flex()
        .flex_col()
        .flex_1()
        .min_h(theme::unit(0.0))
        .child(sheet::header(sheet::Header {
            title: Some(t("YouTube audio").to_owned().into()),
            subtitle: Some(song_name.into()),
            icon: None,
            size: 14.0,
            detail: None,
        }))
        .child(div().w_full().h(theme::unit(1.0)).bg(theme::BORDER))
        .child(body)
        .child(footer);

    sheet::Sheet::new(
        "youtube-editor",
        closing,
        |app, _, cx| {
            app.close_editor(cx);
            cx.notify();
        },
        panel,
    )
    .bar(app.player.read(cx).current().is_some())
    .at_top(move || top.offset().y >= gpui::px(0.0))
    .show(window, cx)
}

fn tab(cx: &mut Context<Jondo>, mode: Mode, glow: f32, icon: Icon) -> impl IntoElement {
    let label = match mode {
        Mode::Assign => t("Assign audio by ID or URL"),
        Mode::Search => t("Search YouTube"),
    };

    hover::hover_in(
        Scope::Shell,
        gpui::ElementId::Name(format!("youtube-mode-{}", icon.name()).into()),
        true,
        move |fade| {
            div()
                .w(theme::unit(40.0))
                .h_full()
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .when(mode == Mode::Search, |this| {
                    this.border_l_1().border_r_1().border_color(theme::BORDER)
                })
                .bg(theme::alpha(theme::ZINC_800, glow))
                .child(icons::icon(
                    icon,
                    16.0,
                    theme::mix(
                        theme::mix(theme::FAINT, theme::ZINC_300, fade),
                        theme::TEXT,
                        glow,
                    ),
                ))
        },
        cx.listener(move |app, _: &gpui::ClickEvent, window, cx| {
            switch(app, mode, window, cx);
            cx.notify();
        }),
    )
    .tooltip(move |window, cx| Tooltip::new(label).build(window, cx))
}

fn clear(cx: &mut Context<Jondo>, assign: bool) -> impl IntoElement {
    let label = t("Clear");
    hover::hover_in(
        Scope::Shell,
        gpui::ElementId::Name(
            if assign {
                "youtube-clear"
            } else {
                "youtube-search-clear"
            }
            .into(),
        ),
        true,
        move |fade| {
            div()
                .w(theme::unit(24.0))
                .h(theme::unit(24.0))
                .mr(theme::unit(8.0))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .rounded(theme::unit(4.0))
                .cursor_pointer()
                .bg(theme::alpha(theme::ZINC_800, fade))
                .child(icons::icon(
                    icons::X,
                    14.0,
                    theme::mix(theme::FAINT, theme::TEXT, fade),
                ))
        },
        cx.listener(move |app, _: &gpui::ClickEvent, window, cx| {
            if assign {
                app.editor.link.clear();
                app.editor.load = false;
                app.editor.resolving = false;
                app.editor.link_at = None;
                app.set_field("youtube-link", "", window, cx);
            } else {
                app.editor.search.clear();
                app.editor.search_at = None;
                let preview = &mut app.preview;
                app.player.update(cx, |player, _| preview.stop(player));
                app.set_field("youtube-search", "", window, cx);
                submit(app, String::new(), cx);
            }

            cx.notify();
        }),
    )
    .tooltip(move |window, cx| Tooltip::new(label).build(window, cx))
}

fn save_button(app: &Jondo, cx: &mut Context<Jondo>, enabled: bool) -> impl IntoElement {
    let fill = app.accent;
    let color = app.binary;
    hover::hover_in(
        Scope::Shell,
        "youtube-save",
        enabled,
        move |fade| {
            div()
                .w(theme::unit(40.0))
                .h(theme::unit(40.0))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .rounded(theme::unit(6.0))
                .bg(fill)
                .opacity(if enabled { 1.0 - 0.2 * fade } else { 0.4 })
                .child(icons::icon(icons::CHECK_BOLD, 16.0, color))
        },
        cx.listener(move |app, _: &gpui::ClickEvent, _, cx| {
            app.editor.link = app.value("youtube-link", cx);
            save(app, cx);
            cx.notify();
        }),
    )
    .tooltip(|window, cx| Tooltip::new(t("Save")).build(window, cx))
}
