use std::ops::Range;
use std::rc::Rc;

use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, App, Context, CursorStyle, Entity, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Window, div,
};

use crate::shared::music::domain::Song;
use crate::shared::music::views::cover;
use crate::shared::ui::views::components::{hover, text};
use crate::shared::ui::views::design::theme;
use crate::shared::ui::views::reorder;
use crate::shell::app::{Jondo, Reorder};
use crate::shell::views::menu::Menu;

use super::super::actions;

#[allow(clippy::too_many_arguments)]
pub fn items(
    entity: Entity<Jondo>,
    full: Rc<Vec<Song>>,
    visible: Rc<Vec<String>>,
    rows: Option<Rc<Vec<usize>>>,
    playlist: Option<String>,
    removable: bool,
    reorder: bool,
    shape: Shape,
) -> impl Fn(Range<usize>, &mut Window, &mut App) -> Vec<AnyElement> + 'static {
    move |range, _window, cx| {
        range
            .filter_map(|slot| {
                let index = match &rows {
                    Some(rows) => *rows.get(slot)?,
                    None => slot,
                };

                let song = full.get(index)?;
                Some(entity.update(cx, |app, cx| {
                    list(
                        app,
                        cx,
                        song,
                        slot,
                        index,
                        &full,
                        &visible,
                        playlist.as_deref(),
                        removable,
                        reorder,
                        shape,
                    )
                    .into_any_element()
                }))
            })
            .collect()
    }
}

#[allow(clippy::too_many_arguments)]
pub fn list(
    app: &mut Jondo,
    cx: &mut Context<Jondo>,
    song: &Song,
    slot: usize,
    index: usize,
    full: &Rc<Vec<Song>>,
    visible: &Rc<Vec<String>>,
    playlist: Option<&str>,
    removable: bool,
    reorder: bool,
    shape: Shape,
) -> impl IntoElement {
    let art = cover::art(
        &mut app.covers,
        song.cover.as_deref().unwrap_or_default(),
        40.0,
        40.0,
        6.0,
    );
    let current = app
        .player
        .read(cx)
        .current()
        .is_some_and(|playing| playing.id == song.id);
    let playing = app.player.read(cx).playing;
    let loading = app.player.read(cx).loading;
    let accent = app.accent;
    let selected = app.ui.selection.iter().any(|id| id == &song.id);
    let broken = song.broken;
    let row_id = gpui::ElementId::Name(
        format!("song-row-{}-{index}", playlist.unwrap_or("collection")).into(),
    );
    let fade = hover::value(&row_id, std::time::Instant::now());
    let dragging = app.ui.reorder.is_some();
    let lit = if dragging {
        0.0
    } else if selected {
        1.0
    } else if broken {
        0.0
    } else {
        fade
    };

    let id = song.id.clone();
    let full = full.clone();
    let visible = visible.clone();
    let credits_scope = format!("row-{}-{index}", playlist.unwrap_or("collection"));
    let playlist = playlist.map(str::to_owned);
    let click_full = full.clone();
    let click_playlist = playlist.clone();
    let trigger_playlist = playlist.clone();
    let trigger_full = full.clone();
    let lifted = reorder && app.ui.drag_from == Some(index);
    let dropping =
        reorder && app.ui.drag_from.is_some() && !lifted && app.ui.drag_to == Some(index);
    // A finger has no hover to reveal the grip, so touch platforms keep it on.
    let hint = reorder && !dragging && (cfg!(target_os = "android") || fade > 0.0);

    let press = cx.listener(
        move |app: &mut Jondo, event: &gpui::MouseDownEvent, _, cx| {
            if reorder && app.ui.selection.is_empty() {
                app.ui.pressed_song = Some((index, event.position));
            }

            cx.stop_propagation();
        },
    );
    let over = cx.listener({
        let playlist = playlist.clone();
        let row = row_id.clone();
        move |app: &mut Jondo, event: &gpui::MouseMoveEvent, _, cx| {
            if !reorder || event.pressed_button != Some(gpui::MouseButton::Left) {
                return;
            }

            let Some((from, start)) = app.ui.pressed_song else {
                return;
            };

            let Some(playlist) = playlist.as_ref() else {
                return;
            };

            if app.ui.drag_from.is_none() {
                if (event.position - start).magnitude() < reorder::DRAG {
                    return;
                }

                hover::set(&row, false, std::time::Instant::now());
                app.ui.reorder = Some(Reorder::Playlist(playlist.clone()));
                app.ui.drag_playlist = Some(playlist.clone());
            }

            app.ui.drag_pointer = Some(event.position);
            let changed = app.ui.drag_from != Some(from) || app.ui.drag_to != Some(index);
            app.ui.drag_from = Some(from);
            app.ui.drag_to = Some(index);
            let scrolling = app.scrolling(cx);
            let edge = scrolling != app.ui.drag_scroll;
            app.ui.drag_scroll = scrolling;
            if changed || edge {
                cx.notify();
            }
        }
    });

    let trigger_id = format!(
        "song-actions-{}-{index}",
        playlist.as_deref().unwrap_or("collection")
    );
    let trigger = actions::trigger(
        cx,
        trigger_id,
        move |app, cx| {
            commands(
                app,
                cx,
                &trigger_full,
                index,
                slot,
                removable,
                trigger_playlist.as_deref(),
            )
        },
        0.0,
    );

    div()
        .id(row_id.clone())
        .relative()
        .w_full()
        .h(theme::unit(shape.height()))
        .flex()
        .items_center()
        .justify_between()
        .gap(theme::unit(if shape == Shape::Video { 12.0 } else { 8.0 }))
        .px(theme::unit(12.0))
        .border_b_1()
        .border_color(theme::ZINC_900)
        .when(!broken, |this| {
            this.cursor(if app.ui.drag_from.is_some() {
                CursorStyle::ClosedHand
            } else {
                CursorStyle::PointingHand
            })
        })
        .bg(theme::alpha(theme::ZINC_800, 0.6 * lit))
        .when(dropping, |this| this.bg(theme::alpha(accent, 0.15)))
        .when(lifted, |this| this.opacity(0.4))
        .when(!broken, |this| {
            this.on_hover(cx.listener({
                let id = row_id.clone();
                move |_, on: &bool, _, cx| {
                    hover::set(&id, *on, std::time::Instant::now());
                    cx.refresh_windows();
                }
            }))
        })
        .on_mouse_down(gpui::MouseButton::Left, press)
        .when(reorder, |this| this.on_mouse_move(over))
        .on_mouse_down(
            gpui::MouseButton::Right,
            cx.listener(move |app, event: &gpui::MouseDownEvent, _, cx| {
                let (header, items) =
                    commands(app, cx, &full, index, slot, removable, playlist.as_deref());
                if items.is_empty() {
                    return;
                }

                app.ui.menu = Some(Menu::context(event.position, header, items, 0.0));
                cx.notify();
            }),
        )
        .on_click(cx.listener(move |app, event: &gpui::ClickEvent, _, cx| {
            if app.ui.drag_from.is_some() {
                cx.stop_propagation();
                return;
            }

            if broken {
                return;
            }

            cx.stop_propagation();
            let touch = cfg!(target_os = "android") || matches!(event, gpui::ClickEvent::Touch(_));
            if touch {
                if app.ui.selection.is_empty() {
                    play(app, click_playlist.as_deref(), &click_full, index, cx);
                } else {
                    app.select_toggle(&id, slot);
                }

                cx.notify();
                return;
            }

            let double =
                matches!(event, gpui::ClickEvent::Mouse(mouse) if mouse.down.click_count >= 2);
            let modifiers = event.modifiers();
            if double {
                app.select_single(&id, slot);
                play(app, click_playlist.as_deref(), &click_full, index, cx);
            } else if modifiers.shift
                && !app.ui.selection.is_empty()
                && let Some(anchor) = app.ui.anchor
            {
                app.select_range(&visible, anchor, slot);
            } else if modifiers.platform || modifiers.control {
                app.select_toggle(&id, slot);
            } else {
                app.select_single(&id, slot);
            }

            if let Some(song) = click_full.get(index) {
                app.identify(song);
            }

            cx.notify();
        }))
        .child(cell(
            app,
            cx,
            song,
            art,
            current,
            loading,
            playing,
            hint,
            accent,
            broken,
            credits_scope,
        ))
        .child(
            div()
                .flex_none()
                .flex()
                .items_center()
                .gap(theme::unit(8.0))
                .when(!broken, |this| {
                    this.child(div().flex().items_center().child(trigger))
                })
                .when(shape == Shape::Song, |this| {
                    this.child(text::numeric(
                        crate::shared::kernel::app::format::time(song.duration as f64),
                        12.0,
                        theme::FAINT,
                    ))
                }),
        )
        .when(reorder, |this| {
            this.child(
                div()
                    .absolute()
                    .inset_0()
                    .border_1()
                    .when(dropping, |ring| {
                        ring.border_color(theme::alpha(accent, 0.5))
                    })
                    .when(!dropping, |ring| {
                        ring.border_color(theme::alpha(accent, 0.0))
                    }),
            )
        })
}

fn play(
    app: &mut Jondo,
    playlist: Option<&str>,
    full: &Rc<Vec<Song>>,
    index: usize,
    cx: &mut Context<Jondo>,
) {
    if let Some(playlist) = playlist {
        app.play_song(playlist, index, cx);
    } else {
        app.play_list(full.as_ref().clone(), index, cx);
    }
}

use super::Shape;
use super::view::{cell, commands};
