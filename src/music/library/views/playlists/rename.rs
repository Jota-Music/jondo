use crate::shared::ui::views::design::theme;

use std::time::{Duration, Instant};

use gpui::{AnyElement, Context, Focusable as _, IntoElement, ParentElement, Styled, Window, div};

use crate::shared::i18n::t;
use crate::shared::ui::views::components::field;
use crate::shell::app::Jondo;

use super::sheet::{accent_button, sheet};

pub(super) fn rename(app: &mut Jondo, window: &mut Window, cx: &mut Context<Jondo>) -> AnyElement {
    let Some((id, name)) = app.ui.rename.clone() else {
        return div().into_any_element();
    };

    let field = app.field("rename", &name, t("Playlist name"), window, cx);
    if app
        .ui
        .dialog_at
        .is_some_and(|at| at.elapsed() < Duration::from_millis(400))
    {
        field.update(cx, |state, cx| state.focus_handle(cx).focus(window, cx));
    }

    let submit = cx.listener(move |app, _: &gpui::ClickEvent, _, cx| {
        let value = app.value("rename", cx);
        if !value.trim().is_empty() {
            app.rename_playlist(&id, &value, cx);
            app.ui.closing = Some(Instant::now());
        }

        cx.notify();
    });

    sheet(
        app,
        "rename-playlist",
        t("Rename playlist"),
        18.0,
        app.ui.closing,
        app.player.read(cx).current().is_some(),
        window,
        cx,
        div()
            .p(theme::unit(24.0))
            .flex()
            .gap(theme::unit(8.0))
            .child(field::text(&field).flex_1())
            .child(accent_button("rename-submit", t("Save"), app, submit)),
    )
}
