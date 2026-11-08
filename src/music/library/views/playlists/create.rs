use crate::shared::ui::views::design::theme;

use std::time::{Duration, Instant};

use gpui::{AnyElement, Context, Focusable as _, ParentElement, Styled, Window, div};

use crate::shared::i18n::t;
use crate::shared::ui::views::components::field;
use crate::shell::app::Jondo;

use super::sheet::{accent_button, sheet};

pub(super) fn create(app: &mut Jondo, window: &mut Window, cx: &mut Context<Jondo>) -> AnyElement {
    let field = app.field("create", "", t("Playlist name"), window, cx);
    if app
        .ui
        .dialog_at
        .is_some_and(|at| at.elapsed() < Duration::from_millis(400))
    {
        field.update(cx, |state, cx| state.focus_handle(cx).focus(window, cx));
    }

    let submit = cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
        let name = app.value("create", cx);
        if !name.trim().is_empty() {
            app.create_playlist(&name, cx);
            app.ui.closing = Some(Instant::now());
        }

        cx.notify();
    });

    sheet(
        "create-playlist",
        t("New playlist"),
        14.0,
        app.ui.closing,
        app.player.read(cx).current().is_some(),
        window,
        cx,
        div()
            .p(theme::unit(16.0))
            .flex()
            .gap(theme::unit(8.0))
            .child(field::text(&field).flex_1())
            .child(accent_button("create-submit", t("Create"), app, submit)),
    )
}
