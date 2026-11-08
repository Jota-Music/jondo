use gpui::prelude::FluentBuilder as _;
use gpui::{
    Context, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement, Styled,
    div,
};

use crate::music::rooms::Room;
use crate::shared::i18n::t;
use crate::shared::music::views::cover;
use crate::shared::ui::views::components::{reveal, text};
use crate::shared::ui::views::design::icons;
use crate::shared::ui::views::design::theme;
use crate::shell::app::Jondo;
use crate::shell::views::menu::{Command, Item, Menu};

use super::badges::status_badge;

/// A saved room drawn like the shelf covers Jota shows: a square tile with the
/// live status in the middle and its actions revealed over the artwork.
pub(super) fn tile(
    app: &mut Jondo,
    cx: &mut Context<Jondo>,
    room: &Room,
    cell: f32,
) -> impl IntoElement {
    let Tile {
        ephemeral,
        mine,
        live,
        connecting,
        peers,
        info,
        known,
        pending,
        needs_password,
        cover_url,
        song_name,
        artist,
        side,
        badge_size,
        group,
        name,
    } = inspect(app, room, cell, cx);
    let id = room.id.clone();
    let target = room.clone();
    let menu_room = room.clone();

    let press = cx.listener({
        let id = id.clone();
        move |app: &mut Jondo, _: &gpui::MouseDownEvent, _, cx| {
            app.rooms_ui.pressed = Some(id.clone());
            cx.notify();
        }
    });
    let over = cx.listener({
        let id = id.clone();
        move |app: &mut Jondo, event: &gpui::MouseMoveEvent, _, cx| {
            if event.pressed_button != Some(gpui::MouseButton::Left) {
                return;
            }

            let Some(from) = app.rooms_ui.pressed.clone() else {
                return;
            };

            if from == id {
                return;
            }

            if app.rooms_ui.drag_from.as_deref() == Some(from.as_str())
                && app.rooms_ui.drag_to.as_deref() == Some(id.as_str())
            {
                return;
            }

            app.rooms_ui.drag_from = Some(from);
            app.rooms_ui.drag_to = Some(id.clone());
            cx.notify();
        }
    });
    let open = cx.listener({
        let target = target.clone();
        move |app: &mut Jondo, _: &gpui::ClickEvent, window, cx| {
            app.edit_room(&target, window, cx);
            cx.notify();
        }
    });
    let remove = cx.listener({
        let id = id.clone();
        move |app: &mut Jondo, _: &gpui::ClickEvent, _, cx| {
            cx.stop_propagation();
            app.forget_room(&id, cx);
            cx.notify();
        }
    });
    let connect = cx.listener({
        let target = target.clone();
        move |app: &mut Jondo, _: &gpui::ClickEvent, _, cx| {
            cx.stop_propagation();
            app.join_room(&target, cx);
            cx.notify();
        }
    });
    let leave = cx.listener(|app: &mut Jondo, _: &gpui::ClickEvent, _, cx| {
        cx.stop_propagation();
        app.stop_room(cx);
        cx.notify();
    });
    let save = cx.listener(|app: &mut Jondo, _: &gpui::ClickEvent, _, cx| {
        cx.stop_propagation();
        app.save_current_room(cx);
        cx.notify();
    });

    let art = if let Some(url) = &cover_url {
        cover::art(&mut app.covers, url, side, side, 6.0)
    } else {
        div()
            .w(theme::unit(side))
            .h(theme::unit(side))
            .flex_none()
            .rounded(theme::unit(6.0))
            .bg(theme::ZINC_900)
            .into_any_element()
    };

    div()
        .id(format!("room-card-{id}"))
        .group(group.clone())
        .w(theme::unit(cell))
        .flex_none()
        .flex()
        .flex_col()
        .rounded(theme::unit(6.0))
        .cursor_pointer()
        .when(!ephemeral, |this| {
            this.on_mouse_down(gpui::MouseButton::Left, press)
                .on_mouse_move(over)
        })
        .when(
            app.rooms_ui.drag_to.as_deref() == Some(id.as_str())
                && app.rooms_ui.drag_from.as_deref() != Some(id.as_str()),
            |this| {
                this.bg(theme::alpha(app.accent, 0.08))
                    .border_1()
                    .border_color(app.accent)
            },
        )
        .on_click(open)
        .on_mouse_down(
            gpui::MouseButton::Right,
            cx.listener(move |app, event: &gpui::MouseDownEvent, _, cx| {
                app.ui.menu = Some(Menu::context(
                    event.position,
                    None,
                    room_menu(&menu_room, ephemeral, live || connecting),
                    0.0,
                ));
                cx.notify();
            }),
        )
        .child(
            div()
                .relative()
                .px(theme::unit(6.0))
                .pt(theme::unit(6.0))
                .child(
                    div()
                        .relative()
                        .w(theme::unit(side))
                        .h(theme::unit(side))
                        .rounded(theme::unit(6.0))
                        .overflow_hidden()
                        .child(art)
                        .when(cover_url.is_some(), |this| {
                            this.child(div().absolute().inset_0().bg(theme::black(0.6)))
                        })
                        .child(
                            div()
                                .absolute()
                                .inset_0()
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(status_badge(
                                    &info,
                                    live,
                                    connecting,
                                    if mine { peers } else { info.members },
                                    pending,
                                    needs_password,
                                    known,
                                    badge_size,
                                )),
                        ),
                )
                .when(!ephemeral, |this| {
                    this.child(
                        div()
                            .absolute()
                            .left(theme::unit(10.0))
                            .top(theme::unit(10.0))
                            .child(reveal::show(
                                &group,
                                reveal::button(
                                    format!("room-remove-{id}"),
                                    icons::TRASH_2,
                                    16.0,
                                    theme::ZINC_300,
                                    Some(theme::black(0.7)),
                                    6.0,
                                    remove,
                                ),
                            )),
                    )
                })
                .child(
                    div()
                        .absolute()
                        .right(theme::unit(10.0))
                        .top(theme::unit(10.0))
                        .child(reveal::show(
                            &group,
                            div()
                                .flex()
                                .gap(theme::unit(4.0))
                                .when(ephemeral, |this| {
                                    this.child(reveal::button(
                                        "room-save-active",
                                        icons::BOOKMARK_PLUS,
                                        16.0,
                                        theme::ZINC_300,
                                        Some(theme::black(0.7)),
                                        6.0,
                                        save,
                                    ))
                                })
                                .child(if mine && (live || connecting) {
                                    reveal::button(
                                        format!("room-leave-{id}"),
                                        icons::LOG_OUT,
                                        16.0,
                                        theme::ZINC_300,
                                        Some(theme::black(0.7)),
                                        6.0,
                                        leave,
                                    )
                                } else {
                                    reveal::button(
                                        format!("room-connect-{id}"),
                                        icons::LOG_IN,
                                        16.0,
                                        theme::ZINC_300,
                                        Some(theme::black(0.7)),
                                        6.0,
                                        connect,
                                    )
                                }),
                        )),
                ),
        )
        .child(label(name, song_name, artist))
}

fn room_menu(room: &Room, ephemeral: bool, connected: bool) -> Vec<Item> {
    let mut items = vec![if connected {
        Item::new(icons::LOG_OUT, t("Leave room"), Command::RoomLeave)
    } else {
        Item::new(
            icons::LOG_IN,
            t("Connect"),
            Command::RoomConnect(room.clone()),
        )
    }];
    if ephemeral {
        items.push(Item::new(
            icons::BOOKMARK_PLUS,
            t("Save room"),
            Command::RoomSave,
        ));
    } else {
        items.push(Item::danger(
            icons::TRASH_2,
            t("Remove"),
            Command::RoomRemove(room.id.clone()),
        ));
    }

    items
}

struct Tile {
    ephemeral: bool,
    mine: bool,
    live: bool,
    connecting: bool,
    peers: usize,
    info: crate::music::rooms::RoomStatus,
    known: bool,
    pending: bool,
    needs_password: bool,
    cover_url: Option<String>,
    song_name: Option<String>,
    artist: Option<String>,
    side: f32,
    badge_size: f32,
    group: String,
    name: String,
}

fn inspect(app: &Jondo, room: &Room, cell: f32, cx: &gpui::App) -> Tile {
    let ephemeral = room.id == super::ACTIVE_ROOM;
    let mine = ephemeral
        || room.id
            == crate::music::rooms::room_id(&app.rooms.read(cx).relay, &app.rooms.read(cx).code);
    let live = mine && app.rooms.read(cx).live();
    let connecting = mine && app.rooms.read(cx).connecting();
    let peers = app.rooms.read(cx).peers;
    let info = app
        .rooms
        .read(cx)
        .statuses
        .get(&room.id)
        .cloned()
        .unwrap_or_default();
    let known = app.rooms.read(cx).statuses.contains_key(&room.id);
    let pending = app.rooms.read(cx).pending_statuses.contains(&room.id);
    let needs_password = info.locked && room.password.is_empty();

    // The room we are in already knows what it plays, so its tile paints that
    // instead of waiting for the next relay poll; every other room reads the
    // relay's cached playback.
    let local = live
        .then(|| {
            app.player.read(cx).current().map(|song| {
                (
                    song.cover.clone(),
                    song.name.clone(),
                    song.artists
                        .first()
                        .cloned()
                        .filter(|name| !name.is_empty()),
                )
            })
        })
        .flatten();
    let cached = info
        .active
        .then_some(info.state.as_ref())
        .flatten()
        .and_then(|playback| playback.song.as_ref());
    let (cover_url, song_name, artist) = match local {
        Some((cover, name, artist)) => (cover, Some(name), artist),
        None => (
            cached.and_then(|song| song.album.covers.first().cloned()),
            cached.map(|song| song.name.clone()),
            cached
                .and_then(|song| song.artists.first())
                .map(|artist| artist.name.clone())
                .filter(|name| !name.is_empty()),
        ),
    };

    let side = (cell - 12.0).max(24.0);
    let badge_size = (side * 0.23).clamp(24.0, 48.0);
    let group = format!("room-card-{}", room.id);
    let name = if room.name.is_empty() {
        room.code.clone()
    } else {
        room.name.clone()
    };

    Tile {
        ephemeral,
        mine,
        live,
        connecting,
        peers,
        info,
        known,
        pending,
        needs_password,
        cover_url,
        song_name,
        artist,
        side,
        badge_size,
        group,
        name,
    }
}

fn label(name: String, song_name: Option<String>, artist: Option<String>) -> gpui::Div {
    div()
        .px(theme::unit(6.0))
        .pt(theme::unit(8.0))
        .pb(theme::unit(6.0))
        .flex()
        .flex_col()
        .gap(theme::unit(2.0))
        .child(text::truncated(
            name,
            14.0,
            theme::TEXT,
            theme::Weight::Medium,
        ))
        .when_some(song_name, |this, name| {
            this.child(text::truncated(
                name,
                12.0,
                theme::FAINT,
                theme::Weight::Regular,
            ))
            .when_some(artist, |this, artist| {
                this.child(text::truncated(
                    artist,
                    12.0,
                    theme::FAINT,
                    theme::Weight::Regular,
                ))
            })
        })
}

#[cfg(test)]
#[path = "card.test.rs"]
mod tests;
