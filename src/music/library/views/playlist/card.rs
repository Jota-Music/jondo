use gpui::prelude::FluentBuilder as _;
use gpui::{
    Context, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement, Styled,
    div,
};

use crate::music::track::views::actions;
use crate::shared::i18n::t;
use crate::shared::music::domain::Playlist;
use crate::shared::music::views::cover;
use crate::shared::ui::views::components::{reveal, text};
use crate::shared::ui::views::design::icons;
use crate::shared::ui::views::design::theme;
use crate::shell::app::{Jondo, Route};
use crate::shell::views::menu::Menu;

pub fn tile(
    app: &mut Jondo,
    cx: &mut Context<Jondo>,
    item: &Playlist,
    width: f32,
    draggable: bool,
    shown: bool,
) -> impl IntoElement {
    let group = format!("tile-{}", item.id);
    let art = if shown {
        cover::playlist(
            &mut app.covers,
            item.cover.as_deref(),
            &item.covers,
            width - 12.0,
            width - 12.0,
            6.0,
        )
    } else {
        cover::placeholder(width - 12.0, width - 12.0, 6.0)
    };

    let subtitle = if item.liked() {
        String::new()
    } else {
        item.subtitle
            .clone()
            .or_else(|| item.owner.clone())
            .filter(|subtitle| !subtitle.is_empty())
            .unwrap_or_else(|| t("{} songs").replace("{}", &item.track_count.to_string()))
    };

    let playable = item.track_count > 0 || !item.songs.is_empty();
    let saved = !item.is_local() && !item.liked() && app.is_saved(&item.id);
    let id = item.id.clone();

    let play = cx.listener({
        let id = item.id.clone();
        move |app: &mut Jondo, _: &gpui::ClickEvent, _, cx| {
            cx.stop_propagation();
            app.play_playlist(&id, cx);
            cx.notify();
        }
    });
    let open = cx.listener(move |app: &mut Jondo, _: &gpui::ClickEvent, _, cx| {
        app.open(Route::Playlist(id.clone()), cx);
        cx.notify();
    });
    let toggle_saved = cx.listener({
        let item = item.clone();
        move |app: &mut Jondo, _: &gpui::ClickEvent, _, cx| {
            cx.stop_propagation();
            app.toggle_saved(&item, cx);
            cx.notify();
        }
    });
    let remove = cx.listener({
        let id = item.id.clone();
        move |app: &mut Jondo, _: &gpui::ClickEvent, _, cx| {
            cx.stop_propagation();
            app.ui.confirm = Some(id.clone());
            cx.notify();
        }
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

            if from == id {
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

    let menu_item = item.clone();
    let selector = item.id.clone();
    div()
        .debug_selector(move || format!("playlist-card-{selector}"))
        .id(format!("tile-{group}"))
        .group(group.clone())
        .w(theme::unit(width))
        .flex_none()
        .flex()
        .flex_col()
        .rounded(theme::unit(6.0))
        .overflow_hidden()
        .cursor_pointer()
        .when(draggable, |this| {
            this.on_mouse_down(gpui::MouseButton::Left, press)
                .on_mouse_move(over)
        })
        .on_mouse_down(
            gpui::MouseButton::Right,
            cx.listener(move |app, event: &gpui::MouseDownEvent, _, cx| {
                let items = actions::shelf(app, &menu_item, cx);
                app.ui.menu = Some(Menu::context(event.position, None, items, 0.0));
                cx.notify();
            }),
        )
        .on_click(open)
        .when(
            draggable
                && app.ui.drag_shelf_to.as_deref() == Some(item.id.as_str())
                && app.ui.drag_shelf_from.as_deref() != Some(item.id.as_str()),
            |this| {
                this.border_1()
                    .border_color(app.accent)
                    .bg(theme::alpha(app.accent, 0.08))
            },
        )
        .child(
            div()
                .relative()
                .px(theme::unit(6.0))
                .pt(theme::unit(6.0))
                .child(art)
                .child(
                    div()
                        .absolute()
                        .left(theme::unit(10.0))
                        .bottom(theme::unit(10.0))
                        .child(cover::badge(item.source, 24.0)),
                )
                .when(saved, |this| {
                    this.child(
                        div()
                            .absolute()
                            .left(theme::unit(10.0))
                            .top(theme::unit(10.0))
                            .child(reveal::show(
                                &group,
                                reveal::button(
                                    format!("shelf-save-{}", item.id),
                                    icons::HEART_FILL,
                                    14.0,
                                    theme::DANGER,
                                    Some(theme::black(0.7)),
                                    6.0,
                                    toggle_saved,
                                ),
                            )),
                    )
                })
                .when(item.is_local(), |this| {
                    this.child(
                        div()
                            .absolute()
                            .left(theme::unit(10.0))
                            .top(theme::unit(10.0))
                            .child(reveal::show(
                                &group,
                                reveal::button(
                                    format!("shelf-remove-{}", item.id),
                                    icons::TRASH_2,
                                    16.0,
                                    theme::white(0.86),
                                    Some(theme::black(0.7)),
                                    6.0,
                                    remove,
                                ),
                            )),
                    )
                })
                .when(playable, |this| {
                    this.child(
                        div()
                            .absolute()
                            .right(theme::unit(10.0))
                            .top(theme::unit(10.0))
                            .child(reveal::show(
                                &group,
                                reveal::button(
                                    format!("shelf-play-{}", item.id),
                                    icons::PLAY,
                                    16.0,
                                    theme::rgb(0xffffff),
                                    Some(theme::black(0.8)),
                                    6.0,
                                    play,
                                ),
                            )),
                    )
                }),
        )
        .child(
            div()
                .px(theme::unit(6.0))
                .pt(theme::unit(8.0))
                .pb(theme::unit(6.0))
                .flex()
                .flex_col()
                .gap(theme::unit(2.0))
                .child(text::truncated(
                    super::title(item),
                    14.0,
                    theme::TEXT,
                    theme::Weight::Medium,
                ))
                .child(text::truncated(
                    subtitle,
                    12.0,
                    theme::FAINT,
                    theme::Weight::Regular,
                )),
        )
}
