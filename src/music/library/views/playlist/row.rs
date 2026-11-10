use gpui::prelude::FluentBuilder as _;
use gpui::{
    Context, CursorStyle, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, div,
};

use crate::music::track::views::actions;
use crate::shared::i18n::t;
use crate::shared::music::domain::Playlist;
use crate::shared::music::views::cover;
use crate::shared::ui::views::components::paint::Scope;
use crate::shared::ui::views::components::{hold, hover, reveal, text};
use crate::shared::ui::views::design::icons;
use crate::shared::ui::views::design::theme;
use crate::shell::app::{Jondo, Route};
use crate::shell::views::menu::Menu;

pub(crate) fn show(
    app: &mut Jondo,
    cx: &mut Context<Jondo>,
    item: &Playlist,
    draggable: bool,
    shown: bool,
) -> impl IntoElement {
    let group = format!("row-{}", item.id);
    let art = if shown {
        cover::playlist(
            &mut app.covers,
            item.cover.as_deref(),
            &item.covers,
            40.0,
            40.0,
            6.0,
        )
    } else {
        cover::placeholder(40.0, 40.0, 6.0)
    };

    let saved = !item.is_local() && !item.liked() && app.is_saved(&item.id);
    let subtitle = if item.liked() {
        String::new()
    } else {
        item.subtitle
            .clone()
            .or_else(|| item.owner.clone())
            .filter(|subtitle| !subtitle.is_empty())
            .unwrap_or_else(|| t("{} songs").replace("{}", &item.track_count.to_string()))
    };

    let id = item.id.clone();
    let play_id = item.id.clone();
    let save_item = item.clone();
    let open = cx.listener(move |app: &mut Jondo, _: &gpui::ClickEvent, _, cx| {
        if app.ui.drag_shelf_from.is_some() {
            // The click stops the mouse up here, so the page never sees it:
            // settle the drag before swallowing the click.
            app.finish_drag(cx);
            cx.stop_propagation();
            cx.notify();
            return;
        }

        app.open(Route::Playlist(id.clone()), cx);
        cx.notify();
    });
    let play = cx.listener(move |app: &mut Jondo, _: &gpui::ClickEvent, _, cx| {
        cx.stop_propagation();
        app.play_playlist(&play_id, cx);
        cx.notify();
    });
    let save = cx.listener(move |app: &mut Jondo, _: &gpui::ClickEvent, _, cx| {
        cx.stop_propagation();
        app.toggle_saved(&save_item, cx);
        cx.notify();
    });
    let press = cx.listener({
        let id = item.id.clone();
        move |app: &mut Jondo, _: &gpui::MouseDownEvent, _, cx| {
            app.ui.pressed_shelf = Some(id.clone());
            cx.notify();
        }
    });
    let over = cx.listener({
        let id = item.id.clone();
        move |app: &mut Jondo, event: &gpui::MouseMoveEvent, _, cx| {
            if event.pressed_button != Some(gpui::MouseButton::Left) {
                return;
            }

            let Some(from) = app.ui.pressed_shelf.clone() else {
                return;
            };

            if from == id && app.ui.drag_shelf_from.is_none() {
                return;
            }

            if app.ui.drag_shelf_from.as_deref() == Some(from.as_str())
                && app.ui.drag_shelf_to.as_deref() == Some(id.as_str())
            {
                return;
            }

            app.ui.drag_shelf_from = Some(from);
            app.ui.drag_shelf_to = Some(id.clone());
            cx.notify();
        }
    });

    let row_id = gpui::ElementId::Name(format!("list-{group}").into());
    let fade = hover::value(&row_id, std::time::Instant::now());
    let menu_item = item.clone();
    let hold_item = item.clone();
    let entity = cx.entity();
    let selector = item.id.clone();
    div()
        .debug_selector(move || format!("playlist-row-{selector}"))
        .id(row_id.clone())
        .group(group.clone())
        .w_full()
        .h(theme::unit(52.0))
        .flex()
        .items_center()
        .rounded(theme::unit(6.0))
        .cursor(if draggable && app.ui.drag_shelf_from.is_some() {
            CursorStyle::ClosedHand
        } else {
            CursorStyle::PointingHand
        })
        .when(fade > 0.0, |this| {
            this.bg(theme::alpha(theme::ZINC_900, fade))
        })
        .on_hover(cx.listener({
            let id = row_id.clone();
            move |app, on: &bool, window, cx| {
                hover::set_in(
                    Scope::Page,
                    &id,
                    *on,
                    std::time::Instant::now(),
                    window.last_input_was_touch(),
                );
                app.paint_page(cx);
            }
        }))
        .when(draggable, |this| {
            this.on_mouse_down(gpui::MouseButton::Left, press)
                .on_mouse_move(over)
        })
        .on_mouse_down(
            gpui::MouseButton::Right,
            cx.listener(move |app, event: &gpui::MouseDownEvent, _, cx| {
                let items = actions::shelf(app, &menu_item, cx);
                app.menu = Some(Menu::context(event.position, None, items, 0.0));
                app.notify_shell(cx);
            }),
        )
        .on_click(open)
        .child(hold::hold(move |position, _, cx| {
            entity.update(cx, |app, cx| {
                let items = actions::shelf(app, &hold_item, cx);
                app.menu = Some(Menu::context(position, None, items, 0.0));
                app.notify_shell(cx);
            });
        }))
        .when(
            draggable
                && app.ui.drag_shelf_to.as_deref() == Some(item.id.as_str())
                && app.ui.drag_shelf_from.as_deref() != Some(item.id.as_str()),
            |this| this.border_1().border_color(app.accent),
        )
        .child(div().w(theme::unit(8.0)))
        .child(art)
        .child(div().w(theme::unit(12.0)))
        .child(
            div()
                .flex_1()
                .min_w(theme::unit(0.0))
                .flex()
                .flex_col()
                .gap(theme::unit(2.0))
                .child(text::truncated(
                    crate::music::library::views::playlist::title(item),
                    14.0,
                    theme::rgb(0xffffff),
                    theme::Weight::Regular,
                ))
                .child(text::truncated(
                    subtitle,
                    12.0,
                    theme::FAINT,
                    theme::Weight::Regular,
                )),
        )
        .when(saved, |this| {
            this.child(
                reveal::show(
                    &group,
                    reveal::button(
                        format!("list-save-{}", item.id),
                        icons::HEART_FILL,
                        14.0,
                        theme::DANGER,
                        None,
                        6.0,
                        save,
                    ),
                )
                .mr(theme::unit(8.0)),
            )
        })
        .child(
            reveal::show(
                &group,
                reveal::button(
                    format!("list-play-{}", item.id),
                    icons::PLAY,
                    14.0,
                    app.binary,
                    Some(app.accent),
                    16.0,
                    play,
                ),
            )
            .mr(theme::unit(10.0)),
        )
}
