use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, div,
};

use crate::shared::i18n::t;
use crate::shared::ui::views::components::paint::Scope;
use crate::shared::ui::views::components::{banner, button, text};
use crate::shared::ui::views::design::icons::{self, Icon};
use crate::shared::ui::views::design::theme;
use crate::shell::app::Jondo;

/// The update banner from Jota: what the release is, the button that installs
/// it, and the X that hides it for the rest of the session.
pub fn show(app: &mut Jondo, cx: &mut Context<Jondo>) -> AnyElement {
    let info = app.settings.read(cx).update.clone();
    let Some(info) = info.filter(|info| info.available && !app.ui.update_dismissed) else {
        return div().into_any_element();
    };

    let accent = app.accent;
    let binary = app.binary;
    let updating = app.ui.updating;
    let installable = info.installable;
    let label = if installable {
        t("Update")
    } else {
        t("View release")
    };

    banner::banner()
        .child(icons::icon(Icon::Download, 16.0, accent))
        .child(
            text::truncated(
                t("Jondo {version} is available").replace("{version}", &info.latest),
                14.0,
                theme::MUTED,
                theme::Weight::Regular,
            )
            .flex_1(),
        )
        .child(if updating {
            div()
                .h(theme::unit(28.0))
                .px(theme::unit(12.0))
                .flex_none()
                .flex()
                .items_center()
                .gap(theme::unit(6.0))
                .rounded_full()
                .bg(theme::ZINC_800)
                .child(button::spinner(12.0, theme::ZINC_300))
                .child(text::weighted(
                    t("Updating…"),
                    12.0,
                    theme::ZINC_300,
                    theme::Weight::SemiBold,
                ))
                .into_any_element()
        } else {
            div()
                .id("update-install")
                .h(theme::unit(28.0))
                .px(theme::unit(12.0))
                .flex_none()
                .flex()
                .items_center()
                .gap(theme::unit(6.0))
                .rounded_full()
                .bg(accent)
                .cursor_pointer()
                .on_click(cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                    app.install_update(cx);
                    cx.notify();
                }))
                .child(icons::icon(
                    if installable {
                        Icon::Download
                    } else {
                        Icon::ExternalLink
                    },
                    12.0,
                    binary,
                ))
                .child(text::weighted(label, 12.0, binary, theme::Weight::SemiBold))
                .into_any_element()
        })
        .child(button::dismiss_in(
            Scope::Shell,
            "update-dismiss",
            cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                app.ui.update_dismissed = true;
                cx.notify();
            }),
        ))
        .into_any_element()
}
