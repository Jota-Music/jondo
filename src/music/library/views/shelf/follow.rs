fn follow_source(follow: &Follow) -> Source {
    follow.source.unwrap_or(match follow.kind {
        FollowKind::Channel => Source::YouTube,
        _ => Source::Spotify,
    })
}

fn follow_menu(local: bool, follow: &Follow) -> Vec<Item> {
    if !local {
        return Vec::new();
    }

    vec![Item::new(
        icons::HEART,
        t("Unfollow"),
        Command::Follow(follow.clone()),
    )]
}

use gpui::prelude::FluentBuilder as _;
use gpui::{
    Context, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement, Styled,
    div,
};

use crate::shared::i18n::t;
use crate::shared::music::domain::{Follow, FollowKind, Source};
use crate::shared::music::views::cover;
use crate::shared::ui::views::components::{button, hold, reveal, text};
use crate::shared::ui::views::design::icons;
use crate::shared::ui::views::design::theme;
use crate::shell::app::Jondo;
use crate::shell::views::menu::{Command, Item, Menu};

pub(super) fn follow_tile(
    app: &mut Jondo,
    cx: &mut Context<Jondo>,
    follow: &Follow,
    cell: f32,
    shown: bool,
) -> impl IntoElement {
    let group = format!("following-{}", follow.id);
    let route = app.follow_route(follow);
    let local = app
        .following()
        .iter()
        .any(|item| item.id.eq_ignore_ascii_case(&follow.id));
    let follow_click = follow.clone();
    let menu_follow = follow.clone();
    let hold_follow = follow.clone();
    let entity = cx.entity();
    let art = if shown {
        cover::art(
            &mut app.covers,
            follow.image.as_deref().unwrap_or_default(),
            cell - 12.0,
            cell - 12.0,
            6.0,
        )
    } else {
        cover::placeholder(cell - 12.0, cell - 12.0, 6.0)
    };

    div()
        .id(format!("following-{}", follow.id))
        .group(group.clone())
        .w(theme::unit(cell))
        .h(theme::unit(cover::tile_height(cell)))
        .flex()
        .flex_col()
        .flex_none()
        .cursor_pointer()
        .on_mouse_down(
            gpui::MouseButton::Right,
            cx.listener(move |app, event: &gpui::MouseDownEvent, _, cx| {
                let items = follow_menu(local, &menu_follow);
                if !items.is_empty() {
                    app.menu = Some(Menu::context(event.position, None, items, 0.0));
                    app.notify_shell(cx);
                }
            }),
        )
        .on_click(cx.listener(move |app, _: &gpui::ClickEvent, _, cx| {
            app.open(route.clone(), cx);
            cx.notify();
        }))
        .child(hold::hold(move |position, _, cx| {
            entity.update(cx, |app, cx| {
                let items = follow_menu(local, &hold_follow);
                if items.is_empty() {
                    return;
                }

                app.menu = Some(Menu::context(position, None, items, 0.0));
                app.notify_shell(cx);
            });
        }))
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
                        .child(cover::badge(follow_source(follow), 24.0)),
                )
                .when(local, |this| {
                    this.child(
                        div()
                            .absolute()
                            .left(theme::unit(10.0))
                            .top(theme::unit(10.0))
                            .child(reveal::show(
                                &group,
                                reveal::button(
                                    format!("unfollow-{}", follow.id),
                                    icons::HEART_FILL,
                                    14.0,
                                    theme::DANGER,
                                    Some(theme::black(0.7)),
                                    6.0,
                                    cx.listener(move |app, _: &gpui::ClickEvent, _, cx| {
                                        cx.stop_propagation();
                                        app.toggle_follow(&follow_click, cx);
                                        cx.notify();
                                    }),
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
                    follow.name.clone(),
                    14.0,
                    theme::TEXT,
                    theme::Weight::Medium,
                ))
                .child(text::truncated(
                    kind_label(follow.kind),
                    12.0,
                    theme::FAINT,
                    theme::Weight::Regular,
                )),
        )
}

pub(super) fn follow_row(
    app: &mut Jondo,
    cx: &mut Context<Jondo>,
    follow: &Follow,
    shown: bool,
) -> impl IntoElement {
    let group = format!("following-row-{}", follow.id);
    let route = app.follow_route(follow);
    let local = app
        .following()
        .iter()
        .any(|item| item.id.eq_ignore_ascii_case(&follow.id));
    let follow_click = follow.clone();
    let menu_follow = follow.clone();
    let hold_follow = follow.clone();
    let entity = cx.entity();
    let art = if shown {
        cover::art(
            &mut app.covers,
            follow.image.as_deref().unwrap_or_default(),
            40.0,
            40.0,
            6.0,
        )
    } else {
        cover::placeholder(40.0, 40.0, 6.0)
    };

    div()
        .id(format!("following-row-{}", follow.id))
        .group(group.clone())
        .w_full()
        .h(theme::unit(52.0))
        .flex()
        .items_center()
        .rounded(theme::unit(6.0))
        .cursor_pointer()
        .hover(|style| style.bg(theme::ZINC_900))
        .on_mouse_down(
            gpui::MouseButton::Right,
            cx.listener(move |app, event: &gpui::MouseDownEvent, _, cx| {
                let items = follow_menu(local, &menu_follow);
                if !items.is_empty() {
                    app.menu = Some(Menu::context(event.position, None, items, 0.0));
                    app.notify_shell(cx);
                }
            }),
        )
        .on_click(cx.listener(move |app, _: &gpui::ClickEvent, _, cx| {
            app.open(route.clone(), cx);
            cx.notify();
        }))
        .child(hold::hold(move |position, _, cx| {
            entity.update(cx, |app, cx| {
                let items = follow_menu(local, &hold_follow);
                if items.is_empty() {
                    return;
                }

                app.menu = Some(Menu::context(position, None, items, 0.0));
                app.notify_shell(cx);
            });
        }))
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
                    follow.name.clone(),
                    14.0,
                    theme::TEXT,
                    theme::Weight::Regular,
                ))
                .child(text::truncated(
                    kind_label(follow.kind),
                    12.0,
                    theme::FAINT,
                    theme::Weight::Regular,
                )),
        )
        .when(local, |this| {
            this.child(reveal::show(
                &group,
                button::icon_flat(
                    format!("unfollow-row-{}", follow.id),
                    icons::HEART_FILL,
                    14.0,
                    theme::DANGER,
                    32.0,
                    true,
                    cx.listener(move |app, _: &gpui::ClickEvent, _, cx| {
                        cx.stop_propagation();
                        app.toggle_follow(&follow_click, cx);
                        cx.notify();
                    }),
                ),
            ))
        })
        .child(div().w(theme::unit(8.0)))
}

fn kind_label(kind: FollowKind) -> String {
    match kind {
        FollowKind::Artist => t("Artist").to_owned(),
        FollowKind::Channel => "YouTube".to_owned(),
        FollowKind::User => "Spotify".to_owned(),
    }
}
