use super::*;
use std::rc::Rc;

use gpui::prelude::FluentBuilder as _;

use gpui::{
    App, Context, CursorStyle, Div, InteractiveElement, IntoElement, ParentElement, Stateful,
    StatefulInteractiveElement, Styled, Window, div,
};

use crate::music::track::views::{actions, credits};
use crate::shared::music::views::cover;
use crate::shared::ui::views::components::paint::Scope;
use crate::shared::ui::views::components::{hover, text};
use crate::shared::ui::views::design::icons;
use crate::shared::ui::views::design::theme;
use crate::shared::ui::views::reorder;
use crate::shell::app::{Jondo, Reorder};
use crate::shell::views::menu::Menu;

pub(super) fn queue_action(
    id: impl Into<gpui::ElementId>,
    icon: icons::Icon,
    enabled: bool,
    live: bool,
    danger: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let base = theme::MUTED;
    let target = if danger {
        theme::rgb(0xfca5a5)
    } else {
        theme::TEXT
    };

    let bg = if danger {
        theme::alpha(theme::rgb(0x450a0a), 0.5)
    } else {
        theme::ZINC_800
    };

    hover::hover_in(
        Scope::Shell,
        id,
        enabled && live,
        move |fade| {
            div()
                .w(theme::unit(30.0))
                .h(theme::unit(30.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded(theme::unit(6.0))
                .when(enabled, |this| {
                    this.bg(theme::mix(theme::white(0.0), bg, fade))
                })
                .when(!enabled, |this| this.opacity(0.3))
                .child(icons::icon(icon, 17.0, theme::mix(base, target, fade)))
        },
        on_click,
    )
    .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
}

pub(super) fn queue_row(
    app: &mut Jondo,
    cx: &mut Context<Jondo>,
    index: usize,
    song: &crate::shared::music::domain::Song,
    current: bool,
    narrow: bool,
    visible: &Rc<Vec<String>>,
) -> Stateful<Div> {
    let art_size = if narrow { 36.0 } else { 40.0 };
    let art = cover::art(
        &mut app.covers,
        song.cover.as_deref().unwrap_or_default(),
        art_size,
        art_size,
        6.0,
    );
    let row_id = gpui::ElementId::Name(format!("queue-row-{index}").into());
    let id = song.id.clone();
    let selected = app.selection.iter().any(|item| item == &id);
    let visible = visible.clone();
    let press = cx.listener(
        move |app: &mut Jondo, event: &gpui::MouseDownEvent, _, cx| {
            app.ui.pressed_queue = None;
            if app.player.read(cx).queue.songs.len() > 1 && app.selection.is_empty() {
                app.ui.pressed_queue = Some((index, event.position));
            }

            let lifted = app.ui.drag_queue_from.take().is_some();
            let targeted = app.ui.drag_queue_to.take().is_some();
            cx.stop_propagation();
            if lifted || targeted {
                cx.notify();
            }
        },
    );
    // A row only knows about itself, so the drop target is the row under the
    // pointer. Moves are reported for every pixel of travel: repainting the
    // window on each one is what made the drag stutter, so only a change in
    // the target or in the edge scroll asks for a frame.
    let over = cx.listener({
        let id = row_id.clone();
        move |app: &mut Jondo, event: &gpui::MouseMoveEvent, _, cx| {
            if event.pressed_button != Some(gpui::MouseButton::Left) {
                return;
            }

            let Some((from, start)) = app.ui.pressed_queue else {
                return;
            };

            if app.ui.drag_queue_from.is_none() {
                if (event.position - start).magnitude() < reorder::DRAG {
                    return;
                }

                hover::set_in(Scope::Shell, &id, false, std::time::Instant::now(), false);
                app.ui.reorder = Some(Reorder::Queue);
            }

            app.ui.drag_pointer = Some(event.position);
            let changed =
                app.ui.drag_queue_from != Some(from) || app.ui.drag_queue_to != Some(index);
            app.ui.drag_queue_from = Some(from);
            app.ui.drag_queue_to = Some(index);
            let scrolling = app.scrolling(cx);
            let edge = scrolling != app.ui.drag_scroll;
            app.ui.drag_scroll = scrolling;
            if changed || edge {
                cx.notify();
            }
        }
    });
    let fade = hover::value(&row_id, std::time::Instant::now());
    let lit = if current { 0.5 } else { 0.3 * fade };
    let lifted = app.ui.drag_queue_from == Some(index);
    let dropping =
        app.ui.drag_queue_from.is_some() && !lifted && app.ui.drag_queue_to == Some(index);
    let live = !app.dragging();
    div()
        .id(row_id.clone())
        .debug_selector(move || format!("queue-row-{index}"))
        .relative()
        .w_full()
        .h(theme::unit(ROW))
        .flex()
        .items_center()
        .gap(theme::unit(8.0))
        .px(theme::unit(if narrow { 4.0 } else { 8.0 }))
        .cursor(if app.ui.drag_queue_from.is_some() {
            CursorStyle::ClosedHand
        } else {
            CursorStyle::PointingHand
        })
        .when(selected, |this| this.bg(theme::alpha(theme::ZINC_800, 0.6)))
        .when(!selected && (current || fade > 0.0), |this| {
            this.bg(theme::alpha(theme::ZINC_900, lit))
        })
        .when(lifted, |this| this.opacity(0.4))
        .when(dropping, |this| this.bg(theme::alpha(app.accent, 0.15)))
        .on_hover(cx.listener({
            let id = row_id.clone();
            move |app, on: &bool, window, cx| {
                hover::set_in(
                    Scope::Shell,
                    &id,
                    *on,
                    std::time::Instant::now(),
                    window.last_input_was_touch(),
                );
                app.notify_shell(cx);
            }
        }))
        .on_mouse_down(gpui::MouseButton::Left, press)
        .on_mouse_down(
            gpui::MouseButton::Right,
            cx.listener(move |app, event: &gpui::MouseDownEvent, _, cx| {
                app.menu = Some(Menu::context(
                    event.position,
                    None,
                    actions::queue(app, cx, index),
                    0.0,
                ));
                app.notify_shell(cx);
            }),
        )
        .on_mouse_move(over)
        .on_click(cx.listener(move |app, event: &gpui::ClickEvent, _, cx| {
            if app.ui.drag_queue_from.is_some() {
                // The click stops the mouse up here, so the sheet never sees
                // it: settle the drag before swallowing the click.
                app.finish_drag(cx);
                cx.stop_propagation();
                cx.notify();
                return;
            }

            cx.stop_propagation();
            let touch = cfg!(target_os = "android") || matches!(event, gpui::ClickEvent::Touch(_));
            if touch {
                if app.selection.is_empty() {
                    app.player.update(cx, |player, _| player.queue.at = index);
                    app.play_current(cx);
                } else {
                    app.select_toggle(&id, index);
                }

                cx.notify();
                return;
            }

            let double =
                matches!(event, gpui::ClickEvent::Mouse(mouse) if mouse.down.click_count >= 2);
            let modifiers = event.modifiers();
            if double {
                app.select_single(&id, index);
                app.player.update(cx, |player, _| player.queue.at = index);
                app.play_current(cx);
            } else if modifiers.shift
                && !app.selection.is_empty()
                && let Some(anchor) = app.anchor
            {
                app.select_range(&visible, anchor, index);
            } else if modifiers.platform || modifiers.control {
                app.select_toggle(&id, index);
            } else {
                app.select_single(&id, index);
            }

            cx.notify();
        }))
        .child(
            div()
                .w(theme::unit(if narrow { 20.0 } else { 24.0 }))
                .mr(theme::unit(12.0))
                .flex()
                .justify_center()
                .child(text::text(
                    (index + 1).to_string(),
                    if narrow { 11.0 } else { 12.0 },
                    theme::ZINC_600,
                )),
        )
        .child(artwork(app, art, current, cx))
        .child(
            div()
                .flex_1()
                .min_w(theme::unit(0.0))
                .flex()
                .flex_col()
                .child(text::truncated(
                    song.name.clone(),
                    if narrow { 13.0 } else { 14.0 },
                    theme::ZINC_100,
                    theme::Weight::Regular,
                ))
                .child(credits::line(
                    app,
                    cx,
                    &format!("queue-{index}"),
                    song,
                    true,
                    if narrow { 11.0 } else { 12.0 },
                    theme::FAINT,
                    theme::Weight::Regular,
                )),
        )
        .child(tail(app, cx, index, narrow, live))
        .child(
            div()
                .absolute()
                .left(theme::unit(0.0))
                .right(theme::unit(0.0))
                .bottom(theme::unit(0.0))
                .h(theme::unit(1.0))
                .bg(theme::alpha(theme::ZINC_900, 0.8)),
        )
        // The ring stays mounted and only changes colour: mounting or
        // unmounting a child on every frame that crosses a row costs about
        // 30ms a frame inside a uniform_list item. It is painted last so the
        // row separator cannot cover its bottom edge.
        .child(
            div()
                .absolute()
                .inset_0()
                .border_1()
                .when(dropping, |ring| {
                    ring.border_color(theme::alpha(app.accent, 0.4))
                })
                .when(!dropping, |ring| {
                    ring.border_color(theme::alpha(app.accent, 0.0))
                }),
        )
}

fn artwork(app: &Jondo, art: gpui::AnyElement, current: bool, cx: &gpui::App) -> Div {
    div().relative().child(art).when(current, |this| {
        this.child(cover::outline(2.0, 6.0, theme::alpha(app.accent, 0.8)))
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(theme::unit(6.0))
                    .bg(theme::black(0.6))
                    .child(if app.player.read(cx).loading {
                        icons::spinning(icons::LOADER, 25.0, app.accent).into_any_element()
                    } else {
                        icons::icon(
                            if app.player.read(cx).playing {
                                icons::MUSIC
                            } else {
                                icons::PAUSE
                            },
                            25.0,
                            app.accent,
                        )
                        .into_any_element()
                    }),
            )
    })
}

fn tail(app: &mut Jondo, cx: &mut Context<Jondo>, index: usize, narrow: bool, live: bool) -> Div {
    {
        let can_down = index + 1 < app.player.read(cx).queue.songs.len();
        let can_snap =
            index != app.player.read(cx).queue.at && index != app.player.read(cx).queue.at + 1;
        let mut actions = div().flex().items_center().gap(theme::unit(2.0));
        if narrow {
            actions = actions.child(actions::trigger_in(
                Scope::Shell,
                cx,
                format!("queue-menu-{index}"),
                28.0,
                move |app, cx| (None, actions::queue(app, cx, index)),
                0.0,
            ));
        } else {
            actions = actions
                .child(queue_action(
                    format!("queue-down-{index}"),
                    icons::CHEVRON_DOWN,
                    can_down,
                    live,
                    false,
                    cx.listener(move |app, _: &gpui::ClickEvent, _, cx| {
                        cx.stop_propagation();
                        app.player
                            .update(cx, |player, _| player.queue.move_song(index, index + 1));
                        app.persist(cx);
                        cx.notify();
                    }),
                ))
                .child(queue_action(
                    format!("queue-snap-{index}"),
                    icons::ARROW_UP_FROM_LINE,
                    can_snap,
                    live,
                    false,
                    cx.listener(move |app, _: &gpui::ClickEvent, _, cx| {
                        cx.stop_propagation();
                        app.player
                            .update(cx, |player, _| player.queue.move_after_current(index));
                        app.persist(cx);
                        cx.notify();
                    }),
                ))
                .child(queue_action(
                    format!("queue-remove-{index}"),
                    icons::TRASH_2,
                    true,
                    live,
                    true,
                    cx.listener(move |app, _: &gpui::ClickEvent, _, cx| {
                        cx.stop_propagation();
                        app.player.update(cx, |player, _| {
                            player.queue.remove(index);
                            if player.queue.songs.is_empty() {
                                player.stop();
                            }
                        });

                        app.persist(cx);
                        cx.notify();
                    }),
                ));
        }

        actions.child(
            div()
                .ml(theme::unit(2.0))
                .when(app.player.read(cx).queue.songs.len() <= 1, |this| {
                    this.opacity(0.3)
                })
                .child(icons::icon(icons::GRIP_VERTICAL, 16.0, theme::ZINC_600)),
        )
    }
}
