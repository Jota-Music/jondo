use gpui::{
    Context, Div, Entity, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Window, div,
};

use gpui_kit::component::input::InputState;

use crate::music::rooms::{self, Role, Status};
use crate::shared::i18n::t;
use crate::shared::ui::views::components::{field, text};
use crate::shared::ui::views::design::icons;
use crate::shared::ui::views::design::theme;
use crate::shell::app::{Jondo, Route};

use super::controls::{action, compact, danger, outline};

pub(super) fn join_panel(app: &mut Jondo, window: &mut Window, cx: &mut Context<Jondo>) -> Div {
    sync(app, window, cx);

    let code = app.rooms.read(cx).draft_code.clone();
    let password = app.rooms.read(cx).draft_password.clone();
    let relay = app.rooms.read(cx).relay.trim().to_owned();
    let current = app.rooms.read(cx).code.clone();
    let active = app.rooms.read(cx).active();
    let live = app.rooms.read(cx).live();
    let connecting = app.rooms.read(cx).connecting();
    let busy = app.rooms.read(cx).status == Status::Connecting;
    let joined = app.rooms.read(cx).joined;
    let peers = app.rooms.read(cx).peers;
    let host = app.rooms.read(cx).role == Role::Host;
    let error = app.rooms.read(cx).error.clone();
    let switching = active && code.trim() != current.trim();
    let ready = !relay.is_empty() && !code.trim().is_empty();
    let saved = ready.then(|| app.saved_room(&code, cx).cloned()).flatten();

    let code_field = app.field("room-code", &code, "party-42", window, cx);
    let password_field = app.secret("room-password", &password, t("Optional"), window, cx);

    let status = if active {
        if live && joined {
            t("{} connected").replace("{}", &peers.to_string())
        } else if live {
            t("Joining...").to_owned()
        } else {
            t("Connecting...").to_owned()
        }
    } else if connecting {
        t("Connecting...").to_owned()
    } else {
        t("Offline").to_owned()
    };

    let tint = if active {
        if live { theme::GREEN } else { theme::AMBER }
    } else {
        theme::MUTED
    };

    let hint = if active {
        t("You are in room {}. Edit the code to switch rooms.").replace("{}", &current)
    } else {
        t("If nobody is hosting it yet, you become the host; otherwise you join and listen in sync.")
            .to_owned()
    };

    let connecting_label = if busy {
        t("Connecting...")
    } else if active {
        t("Switch room")
    } else {
        t("Connect")
    };

    let disabled = !ready || busy || (active && !switching);

    let mut body = div()
        .id("rooms-join-body")
        .flex()
        .flex_col()
        .flex_1()
        .min_h(theme::unit(0.0))
        .overflow_y_scroll()
        .gap(theme::unit(12.0))
        .p(theme::unit(16.0))
        .child(labelled(
            t("Room code"),
            div()
                .flex()
                .items_center()
                .gap(theme::unit(8.0))
                .child(boxed(&code_field, false, true).flex_1())
                .child(compact(
                    "rooms-generate",
                    t("Generate"),
                    icons::REFRESH_CW,
                    cx.listener(|app, _: &gpui::ClickEvent, window, cx| {
                        let value = rooms::code();
                        let password = app.saved_password(&value, cx);
                        app.rooms.update(cx, |rooms, _| {
                            rooms.draft_code = value.clone();
                            rooms.draft_password = password;
                        });
                        app.set_field("room-code", &value, window, cx);
                        let password = app.rooms.read(cx).draft_password.clone();
                        app.set_field("room-password", &password, window, cx);
                        cx.notify();
                    }),
                )),
        ))
        .child(text::text(hint, 12.0, theme::FAINT).mb(theme::unit(8.0)))
        .child(labelled(
            t("Room password"),
            boxed(&password_field, true, false),
        ))
        .child(action(
            "rooms-connect",
            connecting_label,
            icons::LOG_IN,
            disabled,
            cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                let code = app.value("room-code", cx);
                let password = app.value("room-password", cx);
                app.rooms.update(cx, |rooms, _| {
                    rooms.draft_code = code;
                    rooms.draft_password = password;
                });
                app.commit_room_password(cx);
                app.connect_room(cx);
                cx.notify();
            }),
        ));

    if ready && saved.is_none() {
        body = body.child(outline(
            "rooms-save",
            t("Save room"),
            icons::BOOKMARK_PLUS,
            cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                let code = app.value("room-code", cx);
                let password = app.value("room-password", cx);
                app.rooms.update(cx, |rooms, _| {
                    rooms.draft_code = code;
                    rooms.draft_password = password;
                });
                app.save_room(cx);
                cx.notify();
            }),
        ));
    }

    if let Some(room) = saved {
        body = body.child(saved_room(room.id.clone(), cx));
    }

    if active {
        body = body.child(danger(
            "rooms-leave",
            if host {
                t("Stop session")
            } else {
                t("Leave session")
            },
            if host { icons::UNPLUG } else { icons::LOG_OUT },
            cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                app.stop_room(cx);
                cx.notify();
            }),
        ));
    }

    if active && !joined {
        body = body.child(text::text(
            t("Waiting for the room to load..."),
            12.0,
            theme::FAINT,
        ));
    }

    if let Some(error) = error {
        body = body.child(text::text(error, 12.0, theme::DANGER));
    }

    body = body.child(footer(&relay, cx));

    div()
        .flex()
        .flex_col()
        .flex_1()
        .min_h(theme::unit(0.0))
        .child(
            div()
                .flex()
                .items_center()
                .gap(theme::unit(8.0))
                .pl(theme::unit(22.0))
                .pr(theme::unit(56.0))
                .py(theme::unit(12.0))
                .child(icons::icon(icons::TURNTABLE, 22.0, tint))
                .child(text::weighted(
                    t("Rooms"),
                    14.0,
                    theme::TEXT,
                    theme::Weight::Medium,
                ))
                .child(
                    text::truncated(status, 12.0, theme::FAINT, theme::Weight::Regular)
                        .flex_1()
                        .min_w(theme::unit(0.0)),
                ),
        )
        .child(div().w_full().h(theme::unit(1.0)).bg(theme::BORDER))
        .child(body)
}

fn labelled(label: &str, field: impl IntoElement) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(theme::unit(20.0))
        .child(text::text(label.to_owned(), 12.0, theme::FAINT))
        .child(field)
}

fn boxed(state: &Entity<InputState>, masked: bool, mono: bool) -> Div {
    let input = field::input(state);
    let input = if mono {
        input.font_family(theme::MONO)
    } else {
        input
    };

    div()
        .h(theme::unit(40.0))
        .w_full()
        .rounded(theme::unit(8.0))
        .border_1()
        .border_color(theme::BORDER)
        .bg(theme::ZINC_900)
        .px(theme::unit(12.0))
        .flex()
        .items_center()
        .block_mouse_except_scroll()
        .child(if masked { input.mask_toggle() } else { input })
}

/// The form is the source of truth, so opening the sheet from a saved room (or
/// after generating a code) moves the inputs instead of leaving the last typed
/// value in place.
fn sync(app: &mut Jondo, window: &mut Window, cx: &mut Context<Jondo>) {
    let code = app.rooms.read(cx).draft_code.clone();
    if app.value("room-code", cx) != code {
        app.set_field("room-code", &code, window, cx);
    }

    let password = app.rooms.read(cx).draft_password.clone();
    if app.value("room-password", cx) != password {
        app.set_field("room-password", &password, window, cx);
    }
}

fn saved_room(id: String, cx: &mut Context<Jondo>) -> Div {
    div()
        .flex()
        .items_center()
        .gap(theme::unit(8.0))
        .child(
            div()
                .flex_1()
                .h(theme::unit(40.0))
                .px(theme::unit(16.0))
                .rounded(theme::unit(8.0))
                .border_1()
                .border_color(theme::GREEN)
                .flex()
                .items_center()
                .justify_center()
                .gap(theme::unit(8.0))
                .child(icons::icon(icons::BOOKMARK_CHECK, 16.0, theme::GREEN))
                .child(text::text(t("Saved to Rooms"), 14.0, theme::GREEN)),
        )
        .child(
            div()
                .id("rooms-unsave")
                .w(theme::unit(40.0))
                .h(theme::unit(40.0))
                .flex_none()
                .rounded(theme::unit(8.0))
                .border_1()
                .border_color(theme::BORDER)
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .hover(|style| style.bg(theme::ZINC_800))
                .on_click(cx.listener(move |app, _: &gpui::ClickEvent, _, cx| {
                    app.unsave_room(&id, cx);
                    cx.notify();
                }))
                .child(icons::icon(icons::TRASH_2, 16.0, theme::MUTED)),
        )
}

fn footer(relay: &str, cx: &mut Context<Jondo>) -> Div {
    div()
        .flex()
        .items_center()
        .gap(theme::unit(4.0))
        .pt(theme::unit(4.0))
        .child(text::text(
            t("Relay: {url}").replace("{url}", relay),
            12.0,
            theme::ZINC_600,
        ))
        .child(
            div()
                .id("rooms-settings")
                .cursor_pointer()
                .underline()
                .on_click(cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                    app.ui.closing = Some(std::time::Instant::now());
                    app.open(Route::Settings, cx);
                    cx.notify();
                }))
                .child(text::text(t("Settings"), 12.0, theme::ZINC_600)),
        )
}
