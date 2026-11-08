use gpui::prelude::FluentBuilder as _;
use gpui::{Context, Div, IntoElement, ParentElement, Styled, div};

use crate::music::track::views::actions;
use crate::shared::ui::views::components::button;
use crate::shared::ui::views::components::progress::{self, Bar};
use crate::shared::ui::views::design::icons;
use crate::shared::ui::views::design::theme;
use crate::shell::app::{Drag, Jondo};
use crate::shell::views::menu::Command;

pub(super) fn controls(
    app: &mut Jondo,
    cx: &mut Context<Jondo>,
    has_song: bool,
    viewport: f32,
) -> Div {
    let menu = app
        .player
        .read(cx)
        .current()
        .cloned()
        .map(|song| {
            actions::track(
                app,
                cx,
                Vec::new(),
                vec![song],
                Command::Toggle,
                None,
                None,
                None,
                Vec::new(),
            )
        })
        .unwrap_or_default();
    let volume_width = if viewport >= 768.0 { 144.0 } else { 104.0 };
    let minimize = viewport >= 768.0;
    let fill = if has_song {
        app.accent
    } else {
        theme::ZINC_700
    };

    let color = if has_song { app.binary } else { theme::MUTED };
    let (shuffle, repeat, loading, playing) = {
        let player = app.player.read(cx);
        (
            player.queue.shuffle,
            player.repeat,
            player.loading,
            player.playing,
        )
    };

    let shuffle = button::icon_button(
        "ctrl-shuffle",
        icons::SHUFFLE,
        20.0,
        if shuffle {
            app.accent
        } else {
            theme::white(0.6)
        },
        28.0,
        true,
        cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
            app.toggle_shuffle(cx);
            cx.notify();
        }),
    );
    let repeat = button::icon_button(
        "ctrl-repeat",
        if repeat == crate::music::player::domain::queue::Repeat::One {
            icons::REPEAT_1
        } else {
            icons::REPEAT
        },
        20.0,
        if repeat != crate::music::player::domain::queue::Repeat::Off {
            app.accent
        } else {
            theme::white(0.6)
        },
        28.0,
        true,
        cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
            app.cycle_repeat(cx);
            cx.notify();
        }),
    );
    let play = if loading {
        button::circle_spinner(44.0, 20.0, fill, color).into_any_element()
    } else {
        button::circle(
            "ctrl-play",
            if playing { icons::PAUSE } else { icons::PLAY },
            44.0,
            20.0,
            fill,
            color,
            has_song,
            cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                app.toggle(cx);
                cx.notify();
            }),
        )
        .into_any_element()
    };

    let mut row = div()
        .flex()
        .items_center()
        .justify_between()
        .gap(theme::unit(16.0))
        .pt(theme::unit(16.0))
        .pb(theme::unit(8.0))
        .child(
            div()
                .flex()
                .items_center()
                .gap(theme::unit(8.0))
                .child(shuffle)
                .child(repeat),
        );
    // Below 768px the compact bar under the sheet already carries the play
    // button and the track menu, and its volume from 500px up, so the sheet
    // keeps only what the bar lacks.
    let bar = viewport < 768.0;
    if !bar {
        row = row.child(play);
    }

    if !bar && volume_width > 0.0 {
        row = row.child(volume_control(app, cx, volume_width, false, Drag::Controls));
    }

    if bar && viewport < 500.0 {
        row = row.child(volume_inline(app, cx, Drag::Controls));
    }

    row = row
        .child(button::icon_button(
            "ctrl-queue",
            icons::LIST_MUSIC,
            20.0,
            theme::white(0.6),
            32.0,
            true,
            cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                if app.ui.queue_open {
                    app.close_queue();
                } else {
                    app.open_queue(cx);
                }

                cx.notify();
            }),
        ))
        .when(has_song && !bar, |row| {
            row.child(actions::trigger(
                cx,
                "ctrl-more",
                move |_, _| (None, menu.clone()),
                0.0,
            ))
        })
        .when(minimize, |row| {
            row.child(button::icon_button(
                "ctrl-minimize",
                icons::MINIMIZE_2,
                20.0,
                theme::white(0.6),
                28.0,
                true,
                cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                    app.ui.compact = true;
                    app.persist(cx);
                    cx.notify();
                }),
            ))
        });
    row
}

pub(super) fn seek_bar(
    app: &mut Jondo,
    cx: &mut Context<Jondo>,
    thumb: f32,
    enabled: bool,
    kind: Drag,
) -> impl IntoElement {
    let height = progress::HEIGHT;
    let (duration, value) = {
        let player = app.player.read(cx);
        (
            player.duration,
            player.seek_preview.unwrap_or(player.progress),
        )
    };
    let fraction = if duration > 0.0 {
        (value / duration) as f32
    } else {
        0.0
    };

    let dragging = app.ui.drag == Some(kind);
    let accent = if enabled { app.accent } else { theme::ZINC_600 };
    let track = if kind == Drag::Mini {
        theme::NEUTRAL_800
    } else {
        progress::TRACK
    };
    let entity = cx.entity();
    let bar = Bar::new(
        fraction,
        None,
        height,
        accent,
        track,
        thumb,
        enabled,
        dragging,
        move |value, commit, _, cx| {
            entity.update(cx, |app, cx| {
                if commit {
                    app.ui.drag = None;
                    let at = value as f64 * duration;
                    app.player
                        .update(cx, |player, _| player.seek_preview = None);
                    app.seek(at, cx);
                } else {
                    app.ui.drag = Some(kind);
                    app.player.update(cx, |player, _| {
                        player.seek_preview = Some(value as f64 * player.duration);
                    });
                }

                cx.notify();
            });
        },
    );

    if kind == Drag::Mini { bar.flat() } else { bar }
}

pub(crate) fn volume_control(
    app: &mut Jondo,
    cx: &mut Context<Jondo>,
    width: f32,
    compact: bool,
    kind: Drag,
) -> impl IntoElement {
    let size = if compact { 20.0 } else { 24.0 };
    let bar_width = if compact {
        64.0
    } else {
        (width - size - 8.0).max(40.0)
    };

    div()
        .w(theme::unit(width))
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap(theme::unit(8.0))
        .child(volume_button(app, cx, size))
        .child(volume_bar(app, cx, Some(bar_width), kind))
}

pub(super) fn volume_inline(
    app: &mut Jondo,
    cx: &mut Context<Jondo>,
    kind: Drag,
) -> impl IntoElement {
    div()
        .flex_1()
        .min_w(theme::unit(0.0))
        .flex()
        .items_center()
        .gap(theme::unit(8.0))
        .child(volume_button(app, cx, 24.0))
        .child(
            div()
                .flex_1()
                .min_w(theme::unit(0.0))
                .child(volume_bar(app, cx, None, kind)),
        )
}

fn volume_button(app: &mut Jondo, cx: &mut Context<Jondo>, size: f32) -> impl IntoElement {
    let (muted, volume) = {
        let player = app.player.read(cx);
        (player.muted, player.volume)
    };
    let icon = if muted || volume <= 0.001 {
        icons::VOLUME_X
    } else if volume < 1.0 / 3.0 {
        icons::VOLUME
    } else if volume < 2.0 / 3.0 {
        icons::VOLUME_1
    } else {
        icons::VOLUME_2
    };

    button::icon_flat(
        "volume-toggle",
        icon,
        size,
        app.accent,
        size,
        true,
        cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
            app.toggle_mute(cx);
            cx.notify();
        }),
    )
}

fn volume_bar(
    app: &mut Jondo,
    cx: &mut Context<Jondo>,
    width: Option<f32>,
    kind: Drag,
) -> impl IntoElement {
    let value = app.player.read(cx).volume;
    let dragging = app.ui.drag == Some(kind);
    let entity = cx.entity();
    Bar::new(
        value,
        width,
        progress::HEIGHT,
        app.accent,
        progress::TRACK,
        6.0,
        true,
        dragging,
        move |value, commit, _, cx| {
            entity.update(cx, |app, cx| {
                app.ui.drag = if commit { None } else { Some(kind) };
                app.set_volume(value, cx);
                if commit {
                    app.persist(cx);
                }

                cx.notify();
            });
        },
    )
}
