use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Window, div, list,
};

use gpui_kit::component::input::InputEvent;

use crate::shared::i18n::t;
use crate::shared::ui::views::components::scroll;
use crate::shared::ui::views::components::text;
use crate::shared::ui::views::design::theme;
use crate::shell::app::Jondo;

use super::account::account;
use super::backups::{backups, logs};
#[cfg(target_os = "android")]
use super::preferences::playback;
use super::preferences::{about, language, relay};
#[cfg(not(target_os = "android"))]
use super::preferences::{discord, memory};

/// One section per item: the list only builds the ones under the viewport, so
/// a scroll sample does not rebuild the whole page.
#[derive(Clone, Copy)]
enum Item {
    Header,
    Account,
    Language,
    #[cfg(not(target_os = "android"))]
    Discord,
    #[cfg(target_os = "android")]
    Playback,
    #[cfg(not(target_os = "android"))]
    Memory,
    Relay,
    About,
    Backups,
    Logs,
    Footer,
}

#[cfg(target_os = "android")]
const ITEMS: &[Item] = &[
    Item::Header,
    Item::Account,
    Item::Language,
    Item::Playback,
    Item::Relay,
    Item::About,
    Item::Backups,
    Item::Logs,
    Item::Footer,
];

#[cfg(not(target_os = "android"))]
const ITEMS: &[Item] = &[
    Item::Header,
    Item::Account,
    Item::Language,
    Item::Discord,
    Item::Memory,
    Item::Relay,
    Item::About,
    Item::Backups,
    Item::Logs,
    Item::Footer,
];

pub fn show(app: &mut Jondo, window: &mut Window, cx: &mut Context<Jondo>) -> AnyElement {
    let narrow = window.viewport_size().width < theme::unit(768.0);
    let state = app.list("settings", ITEMS.len());
    let entity = cx.entity();

    let page = list(state.clone(), move |ix, window, cx| {
        entity.update(cx, |app, cx| item(app, ix, narrow, window, cx))
    })
    .size_full();

    div()
        .relative()
        .size_full()
        .child(div().size_full().pr(theme::unit(15.0)).child(page))
        .child(scroll::bar(&state))
        .into_any_element()
}

fn item(
    app: &mut Jondo,
    ix: usize,
    narrow: bool,
    window: &mut Window,
    cx: &mut Context<Jondo>,
) -> AnyElement {
    let body = match ITEMS[ix] {
        Item::Header => header(),
        Item::Account => account(app, window, cx),
        Item::Language => language(app, cx),
        #[cfg(not(target_os = "android"))]
        Item::Discord => discord(app, cx),
        #[cfg(target_os = "android")]
        Item::Playback => playback(app, cx),
        #[cfg(not(target_os = "android"))]
        Item::Memory => memory(app, cx),
        Item::Relay => relay(app, window, cx),
        Item::About => about(app, cx),
        Item::Backups => backups(app, narrow, cx),
        Item::Logs => logs(app, cx),
        Item::Footer => footer(cx),
    };

    div()
        .w_full()
        .when(ix + 1 < ITEMS.len(), |this| {
            this.pb(theme::unit(if narrow { 16.0 } else { 24.0 }))
        })
        .debug_selector(move || format!("settings-item-{ix}"))
        .child(body)
        .into_any_element()
}

fn header() -> AnyElement {
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
        ))
        .into_any_element()
}

fn footer(cx: &mut Context<Jondo>) -> AnyElement {
    div()
        .pt(theme::unit(8.0))
        .flex()
        .justify_center()
        .child(
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
        )
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
            cx.notify();
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
