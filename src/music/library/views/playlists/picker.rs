use std::time::Instant;

use crate::shared::i18n::{self, t};
use crate::shared::ui::views::components::{field, text};
use crate::shared::ui::views::design::icons;
use crate::shared::ui::views::design::theme;
use crate::shell::app::Jondo;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, Context, InteractiveElement, ParentElement, StatefulInteractiveElement, Styled,
    Window, div,
};

use super::sheet::{accent_button, outline_button, sheet};

pub(super) fn picker(app: &mut Jondo, window: &mut Window, cx: &mut Context<Jondo>) -> AnyElement {
    if app
        .ui
        .dialog_at
        .is_some_and(|at| at.elapsed() < std::time::Duration::from_millis(50))
    {
        app.ui.picker_targets.clear();
    }

    let songs = app.ui.picker.clone().unwrap_or_default();
    let count = songs.len();
    let name_field = app.field("picker-name", "", t("Playlist name"), window, cx);
    let cancel = cx.listener(|app, _: &gpui::ClickEvent, window, cx| {
        app.ui.closing = Some(Instant::now());
        app.set_field("picker-name", "", window, cx);
        cx.notify();
    });
    let submit = cx.listener({
        let songs = songs.clone();
        move |app, _: &gpui::ClickEvent, window, cx| {
            let name = app.value("picker-name", cx).trim().to_owned();
            let mut targets: Vec<String> = app.ui.picker_targets.drain().collect();
            if !name.is_empty() {
                let id = app.store.create_playlist(&name);
                app.library_changed(cx);
                targets.push(id);
            }

            for id in targets {
                for song in &songs {
                    app.add_to_playlist(&id, song, cx);
                }
            }

            app.set_field("picker-name", "", window, cx);
            app.ui.closing = Some(Instant::now());
            cx.notify();
        }
    });
    let playlists: Vec<_> = app
        .store
        .playlists
        .iter()
        .map(|playlist| (playlist.id.clone(), playlist.name.clone()))
        .collect();

    let picker_scroll = app.pane("playlist-picker");
    let mut rows = div().flex().flex_col();
    if playlists.is_empty() {
        rows = rows.child(
            text::text(t("No playlists yet. Create one below."), 12.0, theme::FAINT)
                .px(theme::unit(8.0))
                .py(theme::unit(12.0)),
        );
    }

    for (id, name) in playlists {
        let selected = app.ui.picker_targets.contains(&id);
        let toggle_id = id.clone();
        rows = rows.child(
            div()
                .id(format!("picker-{id}"))
                .h(theme::unit(40.0))
                .px(theme::unit(12.0))
                .flex()
                .items_center()
                .gap(theme::unit(12.0))
                .rounded(theme::unit(6.0))
                .cursor_pointer()
                .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .when(selected, |this| this.bg(theme::alpha(theme::ZINC_800, 0.7)))
                .when(!selected, |this| {
                    this.hover(|style| style.bg(theme::ZINC_900))
                })
                .on_click(cx.listener(move |app, _: &gpui::ClickEvent, _, cx| {
                    if !app.ui.picker_targets.remove(&toggle_id) {
                        app.ui.picker_targets.insert(toggle_id.clone());
                    }

                    cx.notify();
                }))
                .child(
                    div()
                        .w(theme::unit(20.0))
                        .h(theme::unit(20.0))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(theme::unit(4.0))
                        .border_1()
                        .border_color(if selected {
                            app.accent
                        } else {
                            theme::ZINC_700
                        })
                        .when(selected, |this| this.bg(app.accent))
                        .when(selected, |this| {
                            this.child(icons::icon(icons::CHECK, 14.0, app.binary))
                        }),
                )
                .child(text::truncated(
                    name,
                    14.0,
                    theme::TEXT,
                    theme::Weight::Regular,
                )),
        );
    }

    sheet(
        app,
        "playlist-picker",
        t("Add to playlist"),
        14.0,
        app.ui.closing,
        app.player.read(cx).current().is_some(),
        window,
        cx,
        div()
            .p(theme::unit(8.0))
            .flex()
            .flex_col()
            .gap(theme::unit(8.0))
            .child(text::text(
                i18n::count(count, "1 selected", "{count} selected"),
                12.0,
                theme::FAINT,
            ))
            .child(
                div()
                    .max_h(theme::unit(280.0))
                    .id("picker-scroll")
                    .overflow_y_scroll()
                    .track_scroll(&picker_scroll)
                    .child(rows),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(theme::unit(8.0))
                    .pt(theme::unit(4.0))
                    .child(icons::icon(icons::PLUS, 16.0, theme::FAINT))
                    .child(field::text(&name_field).flex_1()),
            )
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap(theme::unit(8.0))
                    .child(outline_button("picker-cancel", t("Cancel"), app, cancel))
                    .child(accent_button("picker-submit", t("Add"), app, submit)),
            ),
    )
}
