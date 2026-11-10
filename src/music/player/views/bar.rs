use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Window, div,
};

use crate::music::track::views::{actions, credits};
use crate::shared::i18n::t;
use crate::shared::music::views::cover;
use crate::shared::ui::views::components::paint::Scope;
use crate::shared::ui::views::components::{button, hold, text};
use crate::shared::ui::views::design::icons;
use crate::shared::ui::views::design::theme;
use crate::shell::app::{Drag, Jondo};
use crate::shell::views::menu::{Command, Item, Menu};

use super::controls::{seek_bar, volume_control};

pub fn bar(app: &mut Jondo, window: &mut Window, cx: &mut Context<Jondo>) -> AnyElement {
    let narrow = window.viewport_size().width < theme::unit(768.0);
    let viewport = window.viewport_size().width.as_f32();
    let gap = if narrow { 6.0 } else { 12.0 };
    let volume_width = if viewport >= 768.0 {
        144.0
    } else if viewport >= 500.0 {
        104.0
    } else {
        0.0
    };

    let song = app.player.read(cx).current().cloned();
    let has_song = song.is_some();
    let player_menu = song
        .as_ref()
        .map(|song| {
            actions::track(
                app,
                cx,
                Vec::new(),
                vec![song.clone()],
                Command::Toggle,
                None,
                None,
                None,
                Vec::new(),
            )
        })
        .unwrap_or_default();

    let fill = if has_song {
        app.accent
    } else {
        theme::ZINC_700
    };

    let color = if has_song { app.binary } else { theme::MUTED };
    let right = controls(
        app,
        cx,
        narrow,
        gap,
        volume_width,
        fill,
        color,
        has_song,
        player_menu.clone(),
    );
    let left = leading(app, cx, song);
    let hold_menu = player_menu.clone();
    let entity = cx.entity();
    // Same box as the web bar: the seek line in flow, then a 12px padded row
    // around the 48px cover, with the hairline border drawn on top.
    let body = div()
        .relative()
        .w_full()
        .flex_none()
        .bg(theme::BG)
        .on_mouse_down(
            gpui::MouseButton::Right,
            cx.listener(move |app, event: &gpui::MouseDownEvent, _, cx| {
                if player_menu.is_empty() {
                    return;
                }

                app.menu = Some(Menu::context(
                    event.position,
                    None,
                    player_menu.clone(),
                    76.0,
                ));
                app.notify_shell(cx);
            }),
        )
        .child(hold::hold(move |position, _, cx| {
            entity.update(cx, |app, cx| {
                if hold_menu.is_empty() {
                    return;
                }

                app.menu = Some(Menu::context(position, None, hold_menu.clone(), 76.0));
                app.notify_shell(cx);
            });
        }))
        .child(
            div()
                .w_full()
                .child(seek_bar(app, cx, 6.0, has_song, Drag::Mini)),
        )
        .child(
            div()
                .w_full()
                .py(theme::unit(12.0))
                .px(theme::unit(16.0))
                .flex()
                .items_center()
                .gap(theme::unit(gap))
                .child(left)
                .child(right),
        );
    body.into_any_element()
}

#[allow(clippy::too_many_arguments)]
fn controls(
    app: &mut Jondo,
    cx: &mut Context<Jondo>,
    narrow: bool,
    gap: f32,
    volume_width: f32,
    fill: gpui::Rgba,
    color: gpui::Rgba,
    has_song: bool,
    menu: Vec<Item>,
) -> gpui::Div {
    let mut right = div()
        .flex_none()
        .flex()
        .items_center()
        .gap(theme::unit(gap));
    if !narrow {
        right = right
            .child(button::icon_flat_in(
                Scope::Shell,
                "bar-shuffle",
                icons::SHUFFLE,
                20.0,
                if app.player.read(cx).queue.shuffle {
                    app.accent
                } else {
                    theme::white(0.6)
                },
                40.0,
                true,
                cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                    app.toggle_shuffle(cx);
                    app.notify_shell(cx);
                }),
            ))
            .child(button::icon_flat_in(
                Scope::Shell,
                "bar-repeat",
                if app.player.read(cx).repeat == crate::music::player::domain::queue::Repeat::One {
                    icons::REPEAT_1
                } else {
                    icons::REPEAT
                },
                20.0,
                if app.player.read(cx).repeat != crate::music::player::domain::queue::Repeat::Off {
                    app.accent
                } else {
                    theme::white(0.6)
                },
                40.0,
                true,
                cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                    app.cycle_repeat(cx);
                    app.notify_shell(cx);
                }),
            ));
    }

    let mut transport =
        div()
            .flex()
            .items_center()
            .gap(theme::unit(if narrow { 2.0 } else { 4.0 }));
    if volume_width > 0.0 {
        transport = transport
            .child(volume_control(app, cx, volume_width, false, Drag::Volume))
            .child(div().w(theme::unit(8.0)));
    }

    transport = transport
        .child(button::icon_flat_in(
            Scope::Shell,
            "bar-prev",
            icons::SKIP_BACK,
            24.0,
            theme::white(0.8),
            36.0,
            true,
            cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                app.previous(cx);
                cx.notify();
            }),
        ))
        .child(if app.player.read(cx).loading {
            button::circle_spinner(48.0, 24.0, fill, color).into_any_element()
        } else {
            button::circle_in(
                Scope::Shell,
                "bar-play",
                if app.player.read(cx).playing {
                    icons::PAUSE
                } else {
                    icons::PLAY
                },
                48.0,
                24.0,
                fill,
                color,
                has_song,
                cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                    app.toggle(cx);
                    cx.notify();
                }),
            )
            .into_any_element()
        })
        .child(button::icon_flat_in(
            Scope::Shell,
            "bar-next",
            icons::SKIP_FORWARD,
            24.0,
            theme::white(0.8),
            36.0,
            true,
            cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                app.next(cx);
                cx.notify();
            }),
        ));
    right = right.child(transport);

    let mut extras = div()
        .flex()
        .items_center()
        .gap(theme::unit(if narrow { 0.0 } else { 2.0 }));
    if !narrow {
        extras = extras.child(button::icon_flat_in(
            Scope::Shell,
            "bar-queue",
            icons::LIST_MUSIC,
            20.0,
            theme::white(0.6),
            40.0,
            true,
            cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                if app.ui.queue_open {
                    app.close_queue();
                } else {
                    app.open_queue(cx);
                }

                cx.notify();
            }),
        ));
    }

    if has_song {
        extras = extras.child(button::icon_flat_in(
            Scope::Shell,
            "bar-more",
            icons::ELLIPSIS_VERTICAL,
            16.0,
            theme::white(0.6),
            if narrow { 44.0 } else { 32.0 },
            true,
            cx.listener(move |app, event: &gpui::ClickEvent, _, cx| {
                app.menu = Some(Menu::context(event.position(), None, menu.clone(), 76.0));
                app.notify_shell(cx);
            }),
        ));
    }

    let expanded = narrow && app.ui.player_open;
    extras = extras.child(button::icon_flat_in(
        Scope::Shell,
        "bar-expand",
        if expanded {
            icons::CHEVRON_DOWN
        } else {
            icons::CHEVRON_UP
        },
        20.0,
        theme::white(0.6),
        if narrow { 44.0 } else { 40.0 },
        true,
        cx.listener(|app, _: &gpui::ClickEvent, window, cx| {
            if window.viewport_size().width < theme::unit(768.0) {
                if app.ui.player_open {
                    app.close_player();
                } else {
                    app.open_player();
                }
            } else {
                app.ui.compact = false;
                app.persist(cx);
            }

            cx.notify();
        }),
    ));
    right = right.child(extras);
    right
}

fn leading(
    app: &mut Jondo,
    cx: &mut Context<Jondo>,
    song: Option<crate::shared::music::domain::Song>,
) -> AnyElement {
    let empty = song.is_none();
    let left =
        div()
            .id("bar-open")
            .flex_1()
            .min_w(theme::unit(0.0))
            .flex()
            .items_center()
            .gap(theme::unit(12.0))
            .cursor_pointer()
            .on_click(cx.listener(|app, _: &gpui::ClickEvent, window, cx| {
                if window.viewport_size().width < theme::unit(768.0) {
                    app.open_player();
                } else {
                    app.ui.compact = false;
                    app.persist(cx);
                }

                cx.notify();
            }))
            .when_some(song, |left, song| {
                left.child(
                    div()
                        .relative()
                        .child(cover::art(
                            &mut app.covers,
                            song.cover.as_deref().unwrap_or_default(),
                            48.0,
                            48.0,
                            6.0,
                        ))
                        .child(
                            div()
                                .absolute()
                                .inset_0()
                                .rounded(theme::unit(6.0))
                                .border_1()
                                .border_color(theme::white(0.1)),
                        ),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w(theme::unit(0.0))
                        .flex()
                        .flex_col()
                        .debug_selector(|| "bar-title".to_owned())
                        .child(
                            text::truncated(
                                song.name.clone(),
                                14.0,
                                theme::rgb(0xffffff),
                                theme::Weight::Medium,
                            )
                            .line_height(theme::unit(20.0)),
                        )
                        .child(div().h(theme::unit(16.0)).flex().items_center().child(
                            credits::line(
                                app,
                                cx,
                                "bar",
                                &song,
                                false,
                                12.0,
                                theme::white(0.6),
                                theme::Weight::Regular,
                            ),
                        )),
                )
            })
            .when(empty, |left| {
                left.child(
                    text::truncated(
                        t("Nothing playing").to_owned(),
                        14.0,
                        theme::MUTED,
                        theme::Weight::Medium,
                    )
                    .line_height(theme::unit(20.0)),
                )
            });
    left.into_any_element()
}

#[cfg(test)]
#[path = "bar.test.rs"]
mod tests;
