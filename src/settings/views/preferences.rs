use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Window, div,
};

use crate::music::rooms::{self, Status, TokenState};
use crate::shared::i18n::{self, t};
use crate::shared::ui::views::components::{button, switch, text};
use crate::shared::ui::views::design::icons::{self, Icon};
use crate::shared::ui::views::design::theme;
use crate::shell::app::Jondo;
use crate::shell::views::menu::{self, Menu};

use crate::shared::ui::views::components::field;
use crate::shared::ui::views::components::select::{self, Select};

use super::chrome::{card, section};

pub(super) fn language(app: &mut Jondo, cx: &mut Context<Jondo>) -> AnyElement {
    let current = match i18n::locale() {
        i18n::Locale::En => "English",
        i18n::Locale::Es => "Español",
    };

    let open = app
        .menu
        .as_ref()
        .is_some_and(|menu| menu.from(&"settings-language".into()));
    section(
        Icon::Languages,
        t("Language"),
        None,
        div()
            .flex()
            .flex_col()
            .gap(theme::unit(12.0))
            .child(text::text(
                t("Choose the language of the interface."),
                12.0,
                theme::FAINT,
            ))
            .child(select::show(
                Select {
                    id: "settings-language",
                    title: t("Language"),
                    label: current.to_owned(),
                    open,
                    icon: None,
                    chevron: true,
                    full: true,
                    radius: 8.0,
                    border: theme::ZINC_800,
                },
                cx.listener(|app, event: &gpui::ClickEvent, _, cx| {
                    menu::toggle(
                        app,
                        "settings-language",
                        Menu::choice(
                            event.position(),
                            t("Language"),
                            menu::languages(i18n::locale()),
                        ),
                    );
                    app.notify_shell(cx);
                }),
            )),
    )
}

#[cfg(not(target_os = "android"))]
pub(super) fn discord(app: &mut Jondo, cx: &mut Context<Jondo>) -> AnyElement {
    let enabled = app.store.discord;
    section(
        Icon::Headphones,
        t("Discord"),
        Some(
            switch(
                "discord-switch",
                enabled,
                app.accent,
                cx.listener(|app, value: &bool, _, cx| {
                    app.set_discord(*value, cx);
                    cx.notify();
                }),
            )
            .into_any_element(),
        ),
        text::text(
            t("Show what you are listening to in your Discord profile."),
            12.0,
            theme::FAINT,
        ),
    )
}

#[cfg(target_os = "android")]
pub(super) fn playback(app: &mut Jondo, cx: &mut Context<Jondo>) -> AnyElement {
    section(
        Icon::Headphones,
        t("Playback"),
        None,
        div()
            .flex()
            .flex_col()
            .gap(theme::unit(16.0))
            .child(toggle(
                "pause-disconnect",
                "Pause when headphones disconnect",
                "Stop when audio falls back to the phone speaker, like a Bluetooth or wired disconnect.",
                app.store.pause_on_disconnect,
                app.accent,
                cx.listener(|app, value: &bool, _, cx| {
                    app.set_pause_on_disconnect(*value, cx);
                    cx.notify();
                }),
            ))
            .child(toggle(
                "pause-focus",
                "Pause when other apps take audio",
                "Stop for calls and other apps that start playing audio.",
                app.store.pause_on_focus,
                app.accent,
                cx.listener(|app, value: &bool, _, cx| {
                    app.set_pause_on_focus(*value, cx);
                    cx.notify();
                }),
            )),
    )
}

#[cfg(target_os = "android")]
fn toggle(
    id: &'static str,
    title: &'static str,
    body: &'static str,
    checked: bool,
    accent: gpui::Rgba,
    change: impl Fn(&bool, &mut Window, &mut gpui::App) + 'static,
) -> AnyElement {
    div()
        .flex()
        .items_start()
        .justify_between()
        .gap(theme::unit(12.0))
        .child(
            div()
                .flex_1()
                .min_w(theme::unit(0.0))
                .flex()
                .flex_col()
                .gap(theme::unit(4.0))
                .child(text::text(t(title), 13.0, theme::ZINC_100))
                .child(text::text(t(body), 12.0, theme::FAINT)),
        )
        .child(switch(id, checked, accent, change))
        .into_any_element()
}

/// The native counterpart of Jota's idle reload: there is no WebView to reload,
/// so the decoded covers are dropped and the freed pages go back to the kernel.
#[cfg(not(target_os = "android"))]
pub(super) fn memory(app: &mut Jondo, cx: &mut Context<Jondo>) -> AnyElement {
    let enabled = app.store.trim_memory;
    section(
        Icon::Moon,
        t("Background memory"),
        Some(
            switch(
                "memory-switch",
                enabled,
                app.accent,
                cx.listener(|app, value: &bool, _, cx| {
                    app.set_trim_memory(*value, cx);
                    cx.notify();
                }),
            )
            .into_any_element(),
        ),
        text::text(
            t(
                "Free cached covers after the app has been idle for a while to reclaim memory. Playback is not affected.",
            ),
            12.0,
            theme::FAINT,
        ),
    )
}

/// The relay fields save themselves: typing writes the value to the store and
/// queues a check, so a token does not need the field to lose focus.
pub(super) fn wire(app: &mut Jondo, window: &mut Window, cx: &mut Context<Jondo>) {
    use gpui_kit::component::input::InputEvent;

    let url = app.rooms.read(cx).relay.clone();
    let token = app.rooms.read(cx).token.clone();
    let url_field = app.field("relay-url", &url, "relay.example.com", window, cx);
    let token_field = app.secret("relay-token", &token, "", window, cx);

    app.subscriptions.push(cx.subscribe_in(
        &url_field,
        window,
        |app, _, event: &InputEvent, window, cx| match event {
            InputEvent::Change => {
                let value = app.value("relay-url", cx);
                app.rooms.update(cx, |rooms, _| rooms.relay = value);
                app.persist(cx);
                if app.rooms.read(cx).relay.trim().is_empty() {
                    app.rooms.update(cx, |rooms, _| rooms.check_due = None);
                    app.rooms
                        .update(cx, |rooms, _| rooms.verdict = rooms::RelayCheck::default());
                    app.rooms
                        .update(cx, |rooms, _| rooms.token_required = false);
                } else {
                    app.rooms.update(cx, |rooms, _| rooms.schedule_check());
                }

                cx.notify();
            }
            InputEvent::Blur | InputEvent::PressEnter { .. } => {
                if app.rooms.read(cx).relay_locked {
                    return;
                }

                let value = app.value("relay-url", cx).trim().to_owned();
                app.rooms.update(cx, |rooms, _| rooms.relay = value.clone());
                app.set_field("relay-url", &value, window, cx);
                app.persist(cx);
                app.check_relay(cx);
                cx.notify();
            }
            _ => {}
        },
    ));

    app.subscriptions.push(cx.subscribe_in(
        &token_field,
        window,
        |app, _, event: &InputEvent, window, cx| match event {
            InputEvent::Change => {
                let value = app.value("relay-token", cx);
                app.rooms.update(cx, |rooms, _| rooms.token = value);
                app.persist(cx);
                if !app.rooms.read(cx).relay.trim().is_empty() {
                    app.rooms.update(cx, |rooms, _| rooms.schedule_check());
                }

                cx.notify();
            }
            InputEvent::Blur | InputEvent::PressEnter { .. } => {
                if app.rooms.read(cx).relay_locked {
                    return;
                }

                let value = app.value("relay-token", cx).trim().to_owned();
                app.rooms.update(cx, |rooms, _| rooms.token = value.clone());
                app.set_field("relay-token", &value, window, cx);
                app.persist(cx);
                app.check_relay(cx);
                cx.notify();
            }
            _ => {}
        },
    ));
}

pub(super) fn relay(app: &mut Jondo, window: &mut Window, cx: &mut Context<Jondo>) -> AnyElement {
    let url = app.rooms.read(cx).relay.clone();
    if app.value("relay-url", cx) != url {
        app.set_field("relay-url", &url, window, cx);
    }

    let token = app.rooms.read(cx).token.clone();
    if app.value("relay-token", cx) != token {
        app.set_field("relay-token", &token, window, cx);
    }

    let locked = app.rooms.read(cx).relay_locked;
    let required = app.rooms.read(cx).token_required;
    let token_state = app.rooms.read(cx).verdict.token;
    let checking = app.rooms.read(cx).checking || app.rooms.read(cx).check_due.is_some();
    let checked = app.rooms.read(cx).checked_at.is_some();
    let error = app.rooms.read(cx).verdict.error.clone();

    let url_field = app.field("relay-url", &url, "relay.example.com", window, cx);
    let token_field = app.secret("relay-token", &token, "", window, cx);

    let mut body = div()
        .flex()
        .flex_col()
        .gap(theme::unit(12.0))
        .child(text::text(
            t("Relay server that syncs playback between devices. Rooms and their passwords are set from the Rooms panel."),
            12.0,
            theme::FAINT,
        ))
        .child(labelled(
            t("Relay server"),
            field::disabled(&url_field, locked),
        ));

    if locked {
        body = body.child(note(
            Icon::TriangleAlert,
            theme::FAINT,
            t("Pinned by RELAY_API_URL, so changes are not saved."),
        ));
    }

    if let Some(error) = &error {
        body = body.child(note(Icon::CircleAlert, theme::DANGER, error.clone()));
    }

    if required {
        let mut block = div()
            .flex()
            .flex_col()
            .gap(theme::unit(6.0))
            .child(labelled(
                t("Auth token"),
                field::secret_disabled(&token_field, locked, cx),
            ));
        if let Some(line) = token_status(token_state) {
            block = block.child(line);
        }

        body = body.child(block);
    }

    let room = app.rooms.read(cx).code.clone();
    if !room.is_empty() && app.rooms.read(cx).status != Status::Idle {
        let color = if app.rooms.read(cx).status == Status::Open {
            theme::GREEN
        } else {
            theme::AMBER
        };

        let peers = app.rooms.read(cx).peers.to_string();
        body = body.child(
            div()
                .id("relay-session")
                .w_full()
                .flex()
                .items_center()
                .gap(theme::unit(8.0))
                .px(theme::unit(12.0))
                .py(theme::unit(8.0))
                .rounded(theme::unit(8.0))
                .border_1()
                .border_color(theme::ZINC_800)
                .cursor_pointer()
                .hover(|style| style.border_color(theme::ZINC_700))
                .on_click(cx.listener(|app, _: &gpui::ClickEvent, window, cx| {
                    app.open_rooms(window, cx);
                    cx.notify();
                }))
                .child(icons::icon(Icon::Wifi, 14.0, color))
                .child(
                    text::truncated(
                        t("You are in room {}. Edit the code to switch rooms.")
                            .replace("{}", &room),
                        12.0,
                        color,
                        theme::Weight::Regular,
                    )
                    .flex_1(),
                )
                .child(text::numeric(
                    t("{} connected").replace("{}", &peers),
                    12.0,
                    color,
                )),
        );
    }

    section(
        Icon::Turntable,
        t("Rooms"),
        Some(relay_status(
            checking,
            checked,
            !url.trim().is_empty(),
            error.is_some(),
        )),
        body,
    )
}

fn relay_status(checking: bool, checked: bool, has_relay: bool, error: bool) -> AnyElement {
    if checking {
        return div()
            .flex()
            .items_center()
            .gap(theme::unit(6.0))
            .child(button::spinner(12.0, theme::MUTED))
            .child(text::text(t("Checking…"), 12.0, theme::MUTED))
            .into_any_element();
    }

    let (label, color, dot) = if error {
        ("Not connected", theme::DANGER, theme::DANGER)
    } else if checked && has_relay {
        ("Connected", theme::GREEN, theme::GREEN)
    } else {
        ("Not connected", theme::FAINT, theme::ZINC_700)
    };

    div()
        .flex()
        .items_center()
        .gap(theme::unit(6.0))
        .child(
            div()
                .w(theme::unit(6.0))
                .h(theme::unit(6.0))
                .rounded_full()
                .bg(dot),
        )
        .child(text::text(t(label), 12.0, color))
        .into_any_element()
}

fn token_status(token: TokenState) -> Option<AnyElement> {
    let (icon, color, text) = match token {
        TokenState::None => return None,
        TokenState::Required => (
            Icon::TriangleAlert,
            theme::AMBER,
            t("This relay requires an auth token."),
        ),
        TokenState::Accepted => (
            Icon::Check,
            theme::GREEN,
            t("The relay accepted this token."),
        ),
        TokenState::Rejected => (
            Icon::CircleAlert,
            theme::DANGER,
            t("The relay rejected this token."),
        ),
    };

    Some(note(icon, color, text).into_any_element())
}

fn labelled(label: &str, control: impl IntoElement) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .gap(theme::unit(6.0))
        .child(text::weighted(
            label.to_owned(),
            12.0,
            theme::MUTED,
            theme::Weight::Medium,
        ))
        .child(control)
}

fn note(icon: Icon, color: gpui::Rgba, body: impl Into<gpui::SharedString>) -> gpui::Div {
    div()
        .flex()
        .items_start()
        .gap(theme::unit(6.0))
        .child(icons::icon(icon, 14.0, color))
        .child(
            text::text(body, 12.0, color)
                .flex_1()
                .min_w(theme::unit(0.0)),
        )
}

pub(super) fn about(app: &mut Jondo, cx: &mut Context<Jondo>) -> AnyElement {
    let version = env!("CARGO_PKG_VERSION");
    let update = app.settings.read(cx).update.clone();

    let mut card_body = div()
        .flex()
        .items_center()
        .justify_between()
        .gap(theme::unit(12.0))
        .child(
            div()
                .min_w(theme::unit(0.0))
                .flex()
                .flex_col()
                .child(text::text(t("Version"), 12.0, theme::FAINT))
                .child(text::text(format!("v{version}"), 14.0, theme::ZINC_100)),
        );

    let mut actions = div().flex().items_center().gap(theme::unit(8.0));
    if let Some(info) = &update
        && info.available
    {
        let label = if app.ui.updating {
            t("Updating…").to_owned()
        } else if info.installable {
            format!("{} v{}", t("Update to"), info.latest)
        } else {
            format!("{} v{}", t("Download"), info.latest)
        };

        actions = actions.child(
            div()
                .id("about-download")
                .h(theme::unit(32.0))
                .px(theme::unit(12.0))
                .flex()
                .items_center()
                .gap(theme::unit(6.0))
                .rounded(theme::unit(8.0))
                .bg(app.accent)
                .when(app.ui.updating, |this| this.opacity(0.5).cursor_default())
                .when(!app.ui.updating, |this| this.cursor_pointer())
                .on_click(cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                    app.install_update(cx);
                    cx.notify();
                }))
                .child(if app.ui.updating {
                    button::spinner(12.0, app.binary).into_any_element()
                } else {
                    icons::icon(
                        if info.installable {
                            Icon::Download
                        } else {
                            Icon::ExternalLink
                        },
                        12.0,
                        app.binary,
                    )
                    .into_any_element()
                })
                .child(text::weighted(
                    label,
                    12.0,
                    app.binary,
                    theme::Weight::SemiBold,
                )),
        );
    }

    actions = actions.child(
        div()
            .id("about-github")
            .cursor_pointer()
            .on_click(|_, _, _| {
                crate::shared::platform::links::open("https://github.com/Jota-Music/jondo").ok();
            })
            .child(icons::icon(icons::Icon::Github, 18.0, theme::MUTED)),
    );
    card_body = card_body.child(actions);

    section(
        Icon::Info,
        t("About"),
        None,
        div()
            .flex()
            .flex_col()
            .gap(theme::unit(12.0))
            .child(card(card_body))
            .child(text::text(
                t("A minimal, native Jota. GPL-3.0."),
                12.0,
                theme::FAINT,
            )),
    )
}
