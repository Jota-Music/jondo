use super::*;
use std::rc::Rc;

use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement, ScrollStrategy,
    StatefulInteractiveElement, Styled, Window, div, uniform_list,
};

use crate::music::track::views::jump;
use crate::shared::i18n::t;
use crate::shared::ui::views::components::{scroll, text};
use crate::shared::ui::views::design::icons;
use crate::shared::ui::views::design::theme;
use crate::shared::ui::views::reorder;
use crate::shell::app::{Jondo, Reorder};
use crate::shell::views::sheet;

use super::row::*;

pub fn queue(app: &mut Jondo, window: &mut Window, cx: &mut Context<Jondo>) -> AnyElement {
    if let Some(at) = app.ui.queue_closing
        && at.elapsed() >= sheet::DURATION
    {
        app.ui.queue_closing = None;
        cx.notify();
    }

    if !app.ui.queue_open && app.ui.queue_closing.is_none() {
        return div().into_any_element();
    }

    let closing = app.ui.queue_closing;
    let has_song = app.player.read(cx).current().is_some();
    let count = app.player.read(cx).queue.songs.len();
    let current = app.player.read(cx).queue.at;
    let position = (count > 0).then(|| current.min(count - 1) + 1);
    let entity = cx.entity();
    let scroll = app.scroll("queue");
    let top = scroll.clone();
    // A scroll asked for by the handle is already queued for this layout, so
    // the pill stays hidden while it lands and cannot outlive the jump.
    let jumping = scroll.0.borrow().deferred_scroll_to_item.is_some();
    let away = if !jumping && position.is_some() {
        jump::offscreen(&scroll, current, count, 4.0)
    } else {
        None
    };

    let narrow = window.viewport_size().width < theme::unit(640.0);
    let revision = app.player.read(cx).queue.revision;
    let visible = match &app.ui.queue_visible {
        Some((cached, ids)) if *cached == revision => ids.clone(),
        _ => {
            let (revision, ids) = {
                let player = app.player.read(cx);
                (
                    player.queue.revision,
                    Rc::new(
                        player
                            .queue
                            .songs
                            .iter()
                            .map(|song| song.id.clone())
                            .collect::<Vec<_>>(),
                    ),
                )
            };
            app.ui.queue_visible = Some((revision, ids.clone()));
            ids
        }
    };
    let touch = cx.processor(|app: &mut Jondo, event: gpui::LongPressEvent, _, cx| {
        let claimed = app.touch(&Reorder::Queue, &event, cx);
        if claimed {
            cx.notify();
        }

        claimed
    });
    let rows = uniform_list("queue-scroll", count, move |range, _window, cx| {
        range
            .filter_map(|index| {
                entity.update(cx, |app, cx| {
                    let song = app.player.read(cx).queue.songs.get(index)?.clone();
                    Some(
                        queue_row(app, cx, index, &song, current == index, narrow, &visible)
                            .into_any_element(),
                    )
                })
            })
            .collect::<Vec<_>>()
    })
    .track_scroll(&scroll)
    .h_full();

    let body = if count == 0 {
        div()
            .pl(theme::unit(8.0))
            .pr(theme::unit(15.0))
            .pb(theme::unit(12.0))
            .pt(theme::unit(4.0))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap(theme::unit(8.0))
                    .min_h(theme::unit(160.0))
                    .px(theme::unit(16.0))
                    .py(theme::unit(40.0))
                    .text_center()
                    .child(text::text(t("Empty queue."), 14.0, theme::MUTED))
                    .child(text::text(
                        t("Pick a playlist and click a track to start."),
                        12.0,
                        theme::ZINC_600,
                    )),
            )
            .into_any_element()
    } else {
        let limit = window.viewport_size().height.as_f32() * 0.7 - HEADER;
        let pill = away.and_then(|above| {
            let song = app.player.read(cx).queue.songs.get(current)?.clone();
            Some(
                jump::pill(app, "queue-jump", &song, above)
                    .debug_selector(|| "queue-jump".to_owned())
                    .on_click(cx.listener(move |app, _: &gpui::ClickEvent, _, cx| {
                        cx.stop_propagation();
                        app.scroll("queue")
                            .scroll_to_item(current.saturating_sub(1), ScrollStrategy::Top);
                        cx.notify();
                    }))
                    .into_any_element(),
            )
        });
        let gap = if pill.is_some() && jump::ending(&scroll) {
            jump::GAP
        } else {
            12.0
        };

        div()
            .relative()
            .h(theme::unit((count as f32 * ROW).max(FLOOR).min(limit)))
            .min_h(theme::unit(0.0))
            .id("queue-rows")
            .on_click(cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                if !app.selection.is_empty() {
                    app.clear_selection();
                    cx.notify();
                }
            }))
            .child(reorder::Touch::new(touch))
            .child(
                rows.pl(theme::unit(8.0))
                    .pr(theme::unit(15.0))
                    .pb(theme::unit(gap))
                    .pt(theme::unit(4.0)),
            )
            .child(scroll::bar(&scroll))
            .children(pill)
            .into_any_element()
    };

    let content = div()
        .relative()
        .debug_selector(|| "queue-sheet".to_owned())
        .flex()
        .flex_col()
        .flex_1()
        .min_h(theme::unit(0.0))
        // The sheet panel occludes the window, so a mouse up inside it never
        // reaches the root that settles drags elsewhere: the queue settles its
        // own drop here.
        .on_mouse_up(
            gpui::MouseButton::Left,
            cx.listener(|app, _: &gpui::MouseUpEvent, _, cx| {
                if app.ui.drag_queue_from.is_some() {
                    app.finish_drag(cx);
                    cx.notify();
                }
            }),
        )
        .on_mouse_up_out(
            gpui::MouseButton::Left,
            cx.listener(|app, _: &gpui::MouseUpEvent, _, cx| {
                if app.ui.drag_queue_from.is_some() {
                    app.finish_drag(cx);
                    cx.notify();
                }
            }),
        )
        .child(sheet::header(sheet::Header {
            title: None,
            subtitle: None,
            icon: Some((icons::LIST_MUSIC, app.accent)),
            size: 16.0,
            detail: Some(
                text::numeric(
                    match position {
                        Some(position) => format!("{position} / {count}"),
                        None => t("{count} songs").replace("{count}", &count.to_string()),
                    },
                    12.0,
                    theme::MUTED,
                )
                .into_any_element(),
            ),
        }))
        .child(div().w_full().h(theme::unit(1.0)).bg(theme::BORDER))
        .child(body);

    let bar = has_song;
    sheet::Sheet::new(
        "queue",
        closing,
        |app, _, cx| {
            app.close_queue();
            cx.notify();
        },
        content,
    )
    .bar(bar)
    .at_top(move || top.0.borrow().base_handle.offset().y >= gpui::px(0.0))
    .show(window, cx)
}
