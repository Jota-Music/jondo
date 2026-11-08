use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Window, div,
};

use gpui_kit::component::input::InputEvent;

use crate::shared::i18n::t;
use crate::shared::ui::views::components::scroll;
use crate::shared::ui::views::components::text;
use crate::shared::ui::views::design::theme;
use crate::shell::app::Jondo;

use super::account::account;
use super::backups::{backups, logs};
#[cfg(not(target_os = "android"))]
use super::preferences::memory;
use super::preferences::{about, discord, language, relay};

pub fn show(app: &mut Jondo, window: &mut Window, cx: &mut Context<Jondo>) -> AnyElement {
    let narrow = window.viewport_size().width < theme::unit(768.0);
    let scroll = app.pane("settings");
    let mut page = div()
        .id("settings-scroll")
        .size_full()
        .flex()
        .flex_col()
        .gap(theme::unit(if narrow { 16.0 } else { 24.0 }))
        .overflow_y_scroll()
        .track_scroll(&scroll)
        .pr(theme::unit(15.0))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(theme::unit(4.0))
                .child(text::weighted(
                    t("Settings"),
                    20.0,
                    theme::ZINC_100,
                    theme::Weight::SemiBold,
                ))
                .child(text::text(
                    t("Account, interface and backups"),
                    14.0,
                    theme::FAINT,
                )),
        );

    page = page
        .child(account(app, window, cx))
        .child(language(app, cx))
        .child(discord(app, cx));

    #[cfg(not(target_os = "android"))]
    {
        page = page.child(memory(app, cx));
    }

    page = page
        .child(relay(app, window, cx))
        .child(about(app, cx))
        .child(backups(app, narrow, cx))
        .child(logs(app, cx))
        .child(
            div().pt(theme::unit(8.0)).flex().justify_center().child(
                div()
                    .id("settings-developer")
                    .cursor_pointer()
                    .hover(|style| style.underline())
                    .on_click(cx.listener(|_, _: &gpui::ClickEvent, _, _| {
                        crate::shared::platform::links::open("https://github.com/salvadorsru").ok();
                    }))
                    .child(text::text(
                        "Developed by @salvadorsru",
                        12.0,
                        theme::ZINC_600,
                    )),
            ),
        );

    div()
        .relative()
        .size_full()
        .child(page)
        .child(scroll::bar(&scroll))
        .into_any_element()
}

/// The phone's location choice: the folder picker alone cannot reach Drive, so
/// the three sources live in one sheet, like Jota's.
pub fn overlay(app: &mut Jondo, window: &mut Window, cx: &mut Context<Jondo>) -> AnyElement {
    #[cfg(target_os = "android")]
    {
        use crate::shell::app::Route;
        use crate::shell::views::sheet;

        if !matches!(app.route, Route::Settings) {
            app.ui.backup_sheet = false;
            app.ui.backup_closing = None;
            return div().into_any_element();
        }

        if let Some(at) = app.ui.backup_closing
            && at.elapsed() >= sheet::DURATION
        {
            app.ui.backup_sheet = false;
            app.ui.backup_closing = None;
        }

        if !app.ui.backup_sheet {
            return div().into_any_element();
        }

        return super::backups::sources(app, window, cx);
    }

    #[cfg(not(target_os = "android"))]
    {
        let _ = (app, window, cx);
        div().into_any_element()
    }
}

/// The pass the backup uses to seal the sessions it carries is only written
/// once the field loses focus: every keystroke would re-encrypt the document
/// and start a pass.
pub fn wire(app: &mut Jondo, window: &mut Window, cx: &mut Context<Jondo>) {
    super::preferences::wire(app, window, cx);
    let value = app.store.session_pass.clone();
    let pass = app.secret(
        "session-pass",
        &value,
        t("Same pass on every device"),
        window,
        cx,
    );
    app.subscriptions.push(
        cx.subscribe_in(&pass, window, |app, _, event: &InputEvent, _, cx| {
            if !matches!(event, InputEvent::Blur | InputEvent::PressEnter { .. }) {
                return;
            }

            app.set_session_pass(app.value("session-pass", cx).trim().to_owned(), cx);
        }),
    );
}
