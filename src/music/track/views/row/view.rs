use std::rc::Rc;

use gpui::prelude::FluentBuilder as _;
use gpui::{AnyElement, Context, ParentElement, SharedString, Styled, div};

use crate::shared::i18n::t;
use crate::shared::music::domain::Song;
use crate::shared::music::views::cover;
use crate::shared::ui::views::components::text;
use crate::shared::ui::views::design::icons;
use crate::shared::ui::views::design::theme;
use crate::shell::app::Jondo;
use crate::shell::views::menu::{Command, Item};

use super::super::actions;
use super::super::credits;

pub(super) fn commands(
    app: &Jondo,
    cx: &gpui::App,
    full: &Rc<Vec<Song>>,
    index: usize,
    slot: usize,
    removable: bool,
    playlist: Option<&str>,
) -> (Option<SharedString>, Vec<Item>) {
    let Some(song) = full.get(index).cloned() else {
        return (None, Vec::new());
    };

    if song.broken {
        return (None, Vec::new());
    }

    let selected_now = app.ui.selection.iter().any(|id| id == &song.id);
    let batch = selected_now && app.ui.selection.len() > 1;
    let mut targets = vec![song.clone()];
    if batch {
        let chosen: Vec<Song> = app
            .ui
            .selection
            .iter()
            .filter_map(|id| full.iter().find(|song| &song.id == id).cloned())
            .collect();
        if !chosen.is_empty() {
            targets = chosen;
        }
    }

    let count = if batch { targets.len() } else { 0 };
    let header = (count > 1)
        .then(|| SharedString::from(t("{count} selected").replace("{count}", &count.to_string())));
    let play = if batch {
        Command::Play(targets.clone(), 0)
    } else {
        Command::Play(full.as_ref().clone(), index)
    };

    let toggle = (!batch).then(|| (song.id.clone(), slot));
    let remove = if removable {
        playlist.map(|id| {
            Command::Remove(
                id.to_owned(),
                targets.iter().map(|song| song.id.clone()).collect(),
            )
        })
    } else {
        None
    };

    let items = actions::track(
        app,
        cx,
        Vec::new(),
        targets,
        play,
        toggle,
        None,
        remove,
        Vec::new(),
    );
    (header, items)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn cell(
    app: &mut Jondo,
    cx: &mut Context<Jondo>,
    song: &Song,
    art: AnyElement,
    current: bool,
    loading: bool,
    playing: bool,
    hint: bool,
    accent: gpui::Rgba,
    broken: bool,
    credits_scope: String,
) -> gpui::Div {
    div()
        .flex_1()
        .min_w(theme::unit(0.0))
        .flex()
        .items_center()
        .gap(theme::unit(8.0))
        .child(
            div()
                .relative()
                .child(art)
                .when(current, |this| {
                    this.child(cover::outline(2.0, 6.0, accent))
                        .child(cover::current(loading, playing, 6.0, 25.0, accent))
                })
                .when(hint, |this| {
                    this.child(
                        div()
                            .absolute()
                            .top(theme::unit(1.0))
                            .left(theme::unit(1.0))
                            .w(theme::unit(20.0))
                            .h(theme::unit(20.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_full()
                            .border_1()
                            .border_color(theme::ZINC_700)
                            .bg(theme::alpha(theme::ZINC_900, 0.8))
                            .child(icons::icon(icons::GRIP_VERTICAL, 13.0, theme::ZINC_300)),
                    )
                }),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .min_w(theme::unit(0.0))
                .child(text::truncated(
                    song.name.clone(),
                    14.0,
                    if broken {
                        theme::FAINT
                    } else if current {
                        accent
                    } else {
                        theme::rgb(0xffffff)
                    },
                    if current {
                        theme::Weight::Medium
                    } else {
                        theme::Weight::Regular
                    },
                ))
                .when(broken, |this| {
                    this.child(
                        div()
                            .flex()
                            .items_center()
                            .gap(theme::unit(4.0))
                            .child(icons::icon(
                                icons::TRIANGLE_ALERT,
                                12.0,
                                theme::alpha(theme::DANGER, 0.8),
                            ))
                            .child(text::text(
                                t("Unavailable"),
                                12.0,
                                theme::alpha(theme::DANGER, 0.8),
                            )),
                    )
                })
                .when(!broken, |this| {
                    this.child(credits::line(
                        app,
                        cx,
                        &credits_scope,
                        song,
                        true,
                        12.0,
                        if current { accent } else { theme::MUTED },
                        if current {
                            theme::Weight::Medium
                        } else {
                            theme::Weight::Regular
                        },
                    ))
                }),
        )
}
