use gpui::{AnyElement, Context, IntoElement, ParentElement, Styled, Window, div};
use gpui_kit::component::input::InputEvent;

use crate::music::rooms::{self, Room, Status};
use crate::shared::i18n::t;
use crate::shared::ui::views::components::{grid, hint, panel};
use crate::shared::ui::views::design::icons;
use crate::shared::ui::views::design::theme;
use crate::shell::app::{Jondo, Route};
use crate::shell::views::sheet as sheet_ui;

use super::ACTIVE_ROOM;
use super::card::tile;
use super::controls::inline;
use super::join::join_panel;

pub fn wire(app: &mut Jondo, window: &mut Window, cx: &mut Context<Jondo>) {
    let code = app.rooms.read(cx).draft_code.clone();
    let code_field = app.field("room-code", &code, "party-42", window, cx);
    app.subscriptions.push(cx.subscribe_in(
        &code_field,
        window,
        |app, _, event: &InputEvent, window, cx| match event {
            InputEvent::Change => {
                let next = app.value("room-code", cx);
                if next.trim() != app.rooms.read(cx).draft_code.trim() {
                    let password = app.saved_password(&next, cx);
                    app.rooms
                        .update(cx, |rooms, _| rooms.draft_password = password.clone());
                    app.set_field("room-password", &password, window, cx);
                }

                app.rooms.update(cx, |rooms, _| rooms.draft_code = next);
                cx.notify();
            }
            InputEvent::Blur | InputEvent::PressEnter { .. } => {
                let value = app.value("room-code", cx).trim().to_owned();
                app.rooms
                    .update(cx, |rooms, _| rooms.draft_code = value.clone());
                app.set_field("room-code", &value, window, cx);
                cx.notify();
            }
            _ => {}
        },
    ));

    let password = app.rooms.read(cx).draft_password.clone();
    let password_field = app.secret("room-password", &password, t("Optional"), window, cx);
    app.subscriptions.push(cx.subscribe_in(
        &password_field,
        window,
        |app, _, event: &InputEvent, window, cx| match event {
            InputEvent::Change => {
                let password = app.value("room-password", cx);
                app.rooms
                    .update(cx, |rooms, _| rooms.draft_password = password);
                // A saved room keeps the password as it is typed, the way the
                // relay fields do; closing the sheet without blurring must not
                // lose it.
                app.commit_room_password(cx);
                cx.notify();
            }
            InputEvent::Blur | InputEvent::PressEnter { .. } => {
                let value = app.value("room-password", cx).trim().to_owned();
                app.rooms
                    .update(cx, |rooms, _| rooms.draft_password = value.clone());
                app.set_field("room-password", &value, window, cx);
                app.commit_room_password(cx);
                cx.notify();
            }
            _ => {}
        },
    ));
}

pub fn show(app: &mut Jondo, window: &mut Window, cx: &mut Context<Jondo>) -> AnyElement {
    let relay = app.rooms.read(cx).relay.clone();
    if relay.is_empty() {
        return div()
            .size_full()
            .flex()
            .flex_col()
            .rounded(theme::unit(6.0))
            .bg(theme::SURFACE)
            .border_1()
            .border_color(theme::BORDER)
            .child(hint::hint(
                hint::Hint {
                    icon: icons::TURNTABLE,
                    tint: theme::rgb(0xf59e0b),
                    title: t("No relay server yet"),
                    body: t("Rooms are hosted by a relay, so you need one before you can join or host a room."),
                    steps: &[
                        t("Open Settings and go to the Rooms section."),
                        t("Set the relay server address."),
                        t("Come back here and join or host a room."),
                    ],
                },
                Some(hint::action(
                    "rooms-settings",
                    t("Open settings"),
                    Some(icons::SETTINGS),
                    cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                        app.open(Route::Settings, cx);
                        cx.notify();
                    }),
                )),
            ))
            .into_any_element();
    }

    if app.rooms.read(cx).token.trim().is_empty() && app.rooms.read(cx).token_required {
        return div()
            .size_full()
            .flex()
            .flex_col()
            .rounded(theme::unit(6.0))
            .bg(theme::SURFACE)
            .border_1()
            .border_color(theme::BORDER)
            .child(hint::hint(
                hint::Hint {
                    icon: icons::TURNTABLE,
                    tint: theme::rgb(0xf59e0b),
                    title: t("Relay token required"),
                    body: t("This relay requires an auth token, so rooms stay hidden until you add one."),
                    steps: &[
                        t("Open Settings and go to the Rooms section."),
                        t("Enter the relay auth token."),
                        t("Come back here and join or host a room."),
                    ],
                },
                Some(hint::action(
                    "rooms-token-settings",
                    t("Open settings"),
                    Some(icons::SETTINGS),
                    cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                        app.open(Route::Settings, cx);
                        cx.notify();
                    }),
                )),
            ))
            .into_any_element();
    }

    let relay = relay.trim().to_owned();
    let code = app.rooms.read(cx).code.clone();
    let status = app.rooms.read(cx).status;
    let mut page = div()
        .size_full()
        .flex()
        .flex_col()
        .gap(theme::unit(16.0))
        .child(div().flex().items_center().child(inline(
            "rooms-join",
            t("Join or host a room"),
            icons::LOG_IN,
            cx.listener(|app, _: &gpui::ClickEvent, window, cx| {
                app.open_rooms(window, cx);
                cx.notify();
            }),
        )));

    let current = rooms::room_id(&relay, &code);
    let mut items = app.store.rooms_ordered();
    if !code.is_empty() && status != Status::Idle && !items.iter().any(|room| room.id == current) {
        items.insert(
            0,
            Room {
                id: ACTIVE_ROOM.to_owned(),
                code: code.clone(),
                relay_url: relay.clone(),
                token: app.rooms.read(cx).token.trim().to_owned(),
                password: app.rooms.read(cx).password.clone(),
                name: code.clone(),
                saved_at: 0,
            },
        );
    }

    if items.is_empty() {
        page = page.child(panel(hint::plain(t(
            "No saved rooms. Connect to a room and save it from the Rooms panel.",
        ))));
    } else {
        let cell = grid::cell(window);
        let scroll = app.pane("rooms");
        let tiles = items
            .iter()
            .map(|room| tile(app, cx, room, cell).into_any_element())
            .collect();
        page = page.child(panel(grid::show("rooms-scroll", &scroll, window, tiles)));
    }

    page.into_any_element()
}

pub fn overlay(app: &mut Jondo, window: &mut Window, cx: &mut Context<Jondo>) -> AnyElement {
    if !app.rooms.read(cx).show {
        return div().into_any_element();
    }

    let closing = app.ui.closing;
    let body = join_panel(app, window, cx);
    sheet_ui::Sheet::new(
        "rooms-join",
        closing,
        |app, _, cx| {
            app.ui.closing = Some(std::time::Instant::now());
            cx.notify();
        },
        body,
    )
    .bar(app.player.read(cx).current().is_some())
    .show(window, cx)
}
