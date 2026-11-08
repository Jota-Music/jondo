use gpui::{AnyElement, Context, ParentElement, Styled, div};

use crate::shared::i18n::t;
use crate::shared::ui::views::components::text;
use crate::shared::ui::views::design::icons::Icon;
use crate::shared::ui::views::design::theme;
use crate::shell::app::Jondo;

use super::super::chrome::{accent_button, card, section};

pub(crate) fn logs(app: &mut Jondo, cx: &mut Context<Jondo>) -> AnyElement {
    section(
        Icon::FileDown,
        t("Logs"),
        None,
        card(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap(theme::unit(12.0))
                .child(div().min_w(theme::unit(0.0)).child(text::text(
                    t("Save the application log to a file."),
                    12.0,
                    theme::FAINT,
                )))
                .child(accent_button(
                    "save-logs",
                    t("Download"),
                    false,
                    app,
                    cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                        app.save_logs();
                        cx.notify();
                    }),
                )),
        ),
    )
}
