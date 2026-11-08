use std::time::Instant;

use gpui::{AnyElement, Context, ParentElement, Styled, Window, div};

use crate::shared::i18n::t;
use crate::shared::ui::views::components::text;
use crate::shared::ui::views::design::theme;
use crate::shell::app::Jondo;

use super::sheet::{danger_button, outline_button, sheet};

pub(super) fn confirm(
    app: &mut Jondo,
    window: &mut Window,
    cx: &mut Context<Jondo>,
    id: &str,
) -> AnyElement {
    let name = app
        .store
        .playlist(id)
        .map(|playlist| playlist.name.clone())
        .unwrap_or_else(|| t("this playlist").to_owned());
    let id = id.to_owned();
    let remove = cx.listener(move |app, _: &gpui::ClickEvent, _, cx| {
        app.delete_playlist(&id, cx);
        app.ui.closing = Some(Instant::now());
        cx.notify();
    });
    let cancel = cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
        app.ui.closing = Some(Instant::now());
        cx.notify();
    });

    sheet(
        "confirm-delete",
        t("Delete playlist"),
        16.0,
        app.ui.closing,
        app.player.read(cx).current().is_some(),
        window,
        cx,
        div()
            .p(theme::unit(24.0))
            .flex()
            .flex_col()
            .gap(theme::unit(16.0))
            .child(text::text(
                t("Delete “{name}”? This cannot be undone.").replace("{name}", &name),
                14.0,
                theme::MUTED,
            ))
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap(theme::unit(8.0))
                    .child(outline_button("confirm-cancel", t("Cancel"), app, cancel))
                    .child(danger_button("confirm-delete", t("Delete"), app, remove)),
            ),
    )
}

pub(super) fn confirm_remove(
    app: &mut Jondo,
    window: &mut Window,
    cx: &mut Context<Jondo>,
    playlist: &str,
    ids: Vec<String>,
) -> AnyElement {
    let cancel = cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
        app.ui.closing = Some(Instant::now());
        cx.notify();
    });
    let playlist = playlist.to_owned();
    let remove = cx.listener(move |app, _: &gpui::ClickEvent, _, cx| {
        for id in &ids {
            app.remove_song(&playlist, id, cx);
        }

        app.clear_selection();
        app.ui.closing = Some(Instant::now());
        cx.notify();
    });
    sheet(
        "confirm-remove",
        t("Remove from playlist"),
        16.0,
        app.ui.closing,
        app.player.read(cx).current().is_some(),
        window,
        cx,
        div()
            .p(theme::unit(24.0))
            .flex()
            .flex_col()
            .gap(theme::unit(16.0))
            .child(text::text(t("Are you sure?"), 14.0, theme::MUTED))
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap(theme::unit(8.0))
                    .child(outline_button("remove-cancel", t("Cancel"), app, cancel))
                    .child(danger_button("remove-confirm", t("Remove"), app, remove)),
            ),
    )
}
