use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, div,
};

use gpui_kit::component::tooltip::Tooltip;

use crate::shared::i18n::t;
use crate::shared::ui::views::components::{banner, button, text};
use crate::shared::ui::views::design::icons::{self, Icon};
use crate::shared::ui::views::design::theme;
use crate::shell::app::Jondo;
use crate::shell::app::clipboard::Kind;

fn label(kind: Kind) -> &'static str {
    match kind {
        Kind::Playlist => t("Go to this playlist"),
        Kind::Album => t("Go to this album"),
        Kind::Artist => t("Go to this artist"),
        Kind::Channel | Kind::User => t("Go to this channel"),
        Kind::Song => t("Listen to this song"),
    }
}

fn glyph(kind: Kind) -> Icon {
    match kind {
        Kind::Playlist => icons::LIST_MUSIC,
        Kind::Album => icons::DISC_3,
        Kind::Artist => icons::USER_ROUND,
        Kind::Channel => icons::TV,
        Kind::User => icons::USER_ROUND,
        Kind::Song => icons::PLAY,
    }
}

/// The clipboard banner Jota shows when a copied link resolves: the whole row
/// acts, the round button says what it will open, and the X forgets it.
pub fn show(app: &mut Jondo, cx: &mut Context<Jondo>) -> AnyElement {
    let Some(link) = app.clipboard_link(cx).cloned() else {
        return div().into_any_element();
    };

    let busy = app.clipboard.busy;
    let accent = app.accent;
    let binary = app.binary;
    let name = label(link.kind);

    let action = div()
        .id("clipboard-open")
        .flex()
        .flex_1()
        .min_w(theme::unit(0.0))
        .items_center()
        .gap(theme::unit(12.0))
        .when(!busy, |this| {
            this.cursor_pointer()
                .on_click(cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                    app.open_clipboard(cx);
                    cx.notify();
                }))
        })
        .child(icons::icon(icons::LINK_2, 16.0, accent))
        .child(
            text::truncated(
                link.text.clone(),
                14.0,
                theme::MUTED,
                theme::Weight::Regular,
            )
            .flex_1(),
        )
        .child(if busy {
            button::circle_spinner(28.0, 14.0, theme::ZINC_800, theme::ZINC_300).into_any_element()
        } else {
            div()
                .w(theme::unit(28.0))
                .h(theme::unit(28.0))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .rounded_full()
                .bg(accent)
                .child(icons::icon(glyph(link.kind), 14.0, binary))
                .into_any_element()
        })
        .aria_label(name)
        .tooltip(move |window, cx| Tooltip::new(name).build(window, cx));

    banner::banner()
        .child(action)
        .child(button::dismiss(
            "clipboard-dismiss",
            cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                app.dismiss_clipboard();
                cx.notify();
            }),
        ))
        .into_any_element()
}
