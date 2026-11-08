use std::time::Instant;

use gpui::{AnyElement, Context, IntoElement, Window, div};
use gpui_kit::component::input::InputEvent;

use crate::shared::i18n::t;
use crate::shell::app::Jondo;
use crate::shell::views::sheet as sheet_ui;

use super::confirm::{confirm, confirm_remove};
use super::create::create;
use super::picker::picker;
use super::rename::rename;

pub fn show(app: &mut Jondo, window: &mut Window, cx: &mut Context<Jondo>) -> AnyElement {
    if let Some(at) = app.ui.closing
        && at.elapsed() >= sheet_ui::DURATION
    {
        app.ui.create_open = false;
        app.ui.rename = None;
        app.ui.confirm = None;
        app.ui.removing = None;
        app.ui.picker = None;
        app.ui.choice = None;
        app.ui.editor = Default::default();
        app.rooms_ui.confirm = None;
        app.rooms.update(cx, |rooms, _| rooms.show = false);
        app.ui.closing = None;
    }

    let open = app.ui.create_open
        || app.ui.rename.is_some()
        || app.ui.confirm.is_some()
        || app.ui.removing.is_some()
        || app.ui.picker.is_some()
        || app.ui.editor.open;
    if open {
        if app.ui.dialog_at.is_none() {
            app.ui.dialog_at = Some(Instant::now());
        }
    } else {
        app.ui.dialog_at = None;
    }

    if app.ui.create_open {
        return create(app, window, cx);
    }

    if app.ui.rename.is_some() {
        return rename(app, window, cx);
    }

    if let Some(id) = app.ui.confirm.clone() {
        return confirm(app, window, cx, &id);
    }

    if let Some((playlist, ids)) = app.ui.removing.clone() {
        return confirm_remove(app, window, cx, &playlist, ids);
    }

    if app.ui.picker.is_some() {
        return picker(app, window, cx);
    }

    if app.ui.editor.open {
        return crate::music::track::views::youtube::show(app, window, cx).into_any_element();
    }

    div().into_any_element()
}

pub fn wire(app: &mut Jondo, window: &mut Window, cx: &mut Context<Jondo>) {
    let create = app.field("create", "", t("Playlist name"), window, cx);
    app.subscriptions.push(cx.subscribe_in(
        &create,
        window,
        |app, _, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::PressEnter { .. }) {
                let name = app.value("create", cx);
                if !name.trim().is_empty() {
                    app.create_playlist(&name, cx);
                    app.ui.closing = Some(Instant::now());
                }

                cx.notify();
            }
        },
    ));

    let channel = app.field("channel-search", "", t("Search in channel"), window, cx);
    app.subscriptions.push(cx.subscribe_in(
        &channel,
        window,
        |app, state, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::Change) {
                app.ui.channel_query = state.read(cx).value().to_string();
                cx.notify();
            }
        },
    ));

    let rename = app.field("rename", "", t("Playlist name"), window, cx);
    app.subscriptions.push(cx.subscribe_in(
        &rename,
        window,
        |app, _, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::PressEnter { .. })
                && let Some((id, _)) = app.ui.rename.clone()
            {
                let value = app.value("rename", cx);
                if !value.trim().is_empty() {
                    app.rename_playlist(&id, &value, cx);
                    app.ui.closing = Some(Instant::now());
                }

                cx.notify();
            }
        },
    ));
}
