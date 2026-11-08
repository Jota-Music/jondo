use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, Context, Div, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, div,
};

use crate::music::track::views::credits;
use crate::shared::i18n::t;
use crate::shared::kernel::app::format::time;
use crate::shared::ui::views::components::text;
use crate::shared::ui::views::design::icons;
use crate::shared::ui::views::design::theme;
use crate::shell::app::{Drag, Jondo};

use super::controls::{controls, seek_bar};

pub(super) fn info(app: &mut Jondo, cx: &mut Context<Jondo>, has_song: bool, viewport: f32) -> Div {
    let song = app.player.read(cx).current().cloned();
    let name = song
        .as_ref()
        .map_or_else(|| t("Nothing playing").to_owned(), |song| song.name.clone());
    let duration = match &song {
        Some(_) if app.player.read(cx).duration > 0.0 => app.player.read(cx).duration,
        Some(song) => song.duration as f64,
        None => 0.0,
    };

    let shown = app
        .player
        .read(cx)
        .seek_preview
        .unwrap_or(app.player.read(cx).progress);

    let mut column = div().flex().flex_col();
    if let Some(song) = &song
        && let Some(line) = credits::album(
            app,
            cx,
            "player-album",
            song,
            12.0,
            theme::white(0.59),
            theme::Weight::Regular,
        )
    {
        column = column.child(div().h(theme::unit(16.0)).flex().items_center().child(line));
    }

    column = column.child(
        text::truncated(name, 20.0, theme::rgb(0xffffff), theme::Weight::SemiBold)
            .line_height(theme::unit(28.0)),
    );
    if let Some(song) = &song
        && !song.artists.is_empty()
    {
        column = column.child(div().h(theme::unit(20.0)).flex().items_center().child(
            credits::line(
                app,
                cx,
                "player-artists",
                song,
                false,
                14.0,
                theme::white(0.7),
                theme::Weight::Regular,
            ),
        ));
    }

    if has_song {
        column = column.child(div().mt(theme::unit(8.0)).child(youtube_row(app, cx)));
    }

    let times = div()
        .flex()
        .items_center()
        .justify_between()
        .mt(theme::unit(6.0))
        .child(text::numeric(time(shown), 14.0, theme::white(0.7)).line_height(theme::unit(20.0)))
        .child(
            text::numeric(time(duration), 14.0, theme::white(0.7)).line_height(theme::unit(20.0)),
        );
    let row = controls(app, cx, has_song, viewport);

    if viewport >= 768.0 {
        column
            .child(
                div()
                    .mt(theme::unit(12.0))
                    .child(seek_bar(app, cx, 6.0, has_song, Drag::Seek)),
            )
            .child(times)
            .child(row)
    } else {
        // On the phone sheet the seek line belongs to the compact player bar
        // underneath, so the times sit at the bottom edge to read as its labels.
        column.child(row).child(times)
    }
}

fn youtube_row(app: &mut Jondo, cx: &mut Context<Jondo>) -> AnyElement {
    let Some(song) = app.player.read(cx).current().cloned() else {
        return div().into_any_element();
    };

    let id = song.video().unwrap_or_default().to_owned();
    let label = if id.is_empty() {
        "...".to_owned()
    } else {
        id.clone()
    };

    div()
        .id("youtube-chip")
        .h(theme::unit(32.0))
        .flex()
        .items_center()
        .gap(theme::unit(8.0))
        .cursor_pointer()
        .on_click(cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
            crate::music::track::views::youtube::open(app, cx);
            cx.notify();
        }))
        .child(icons::icon(icons::Icon::YouTube, 16.0, theme::white(0.2)))
        .child(
            div()
                .h(theme::unit(20.0))
                .px(theme::unit(6.0))
                .flex()
                .items_center()
                .rounded(theme::unit(4.0))
                .bg(theme::white(0.1))
                .child(
                    text::text(label, 12.0, theme::white(0.5))
                        .font_family(theme::MONO)
                        .line_height(theme::unit(16.0)),
                ),
        )
        .when(!id.is_empty(), |this| {
            this.child(
                div()
                    .w(theme::unit(24.0))
                    .h(theme::unit(24.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(icons::icon(icons::EDIT_2, 16.0, theme::white(0.5))),
            )
        })
        .into_any_element()
}
