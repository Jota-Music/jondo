use std::rc::Rc;

use gpui::{
    Context, InteractiveElement, IntoElement, ParentElement, Styled, Window, div, uniform_list,
};

use crate::music::library::views::playlist::{card, row};
use crate::shared::music::domain::{Follow, Playlist};
use crate::shared::music::views::cover;
use crate::shared::ui::views::components::{grid, loading, scroll};
use crate::shared::ui::views::design::theme;
use crate::shell::app::{Filter, Jondo, SpotifyState};

use super::controls::controls;
use super::empty::empty;
use super::follow::{follow_row, follow_tile};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Scope {
    Library,
    Following,
}

pub fn show(app: &mut Jondo, window: &mut Window, cx: &mut Context<Jondo>) -> impl IntoElement {
    render(app, window, cx, Scope::Library)
}

pub fn following(
    app: &mut Jondo,
    window: &mut Window,
    cx: &mut Context<Jondo>,
) -> impl IntoElement {
    render(app, window, cx, Scope::Following)
}

enum Items {
    Library(Rc<Vec<Playlist>>),
    Following(Rc<Vec<Follow>>),
}

impl Items {
    fn is_empty(&self) -> bool {
        match self {
            Self::Library(items) => items.is_empty(),
            Self::Following(items) => items.is_empty(),
        }
    }

    fn len(&self) -> usize {
        match self {
            Self::Library(items) => items.len(),
            Self::Following(items) => items.len(),
        }
    }
}

pub(super) fn filter(app: &Jondo, scope: Scope) -> Filter {
    match scope {
        Scope::Library => app.ui.filter,
        Scope::Following => app.ui.following_filter,
    }
}

pub(super) fn set_filter(app: &mut Jondo, scope: Scope, value: Filter) {
    match scope {
        Scope::Library => app.ui.filter = value,
        Scope::Following => app.ui.following_filter = value,
    }
}

pub(super) fn grid(app: &Jondo, scope: Scope) -> bool {
    match scope {
        Scope::Library => app.ui.grid,
        Scope::Following => app.ui.following_grid,
    }
}

pub(super) fn set_grid(app: &mut Jondo, scope: Scope, value: bool) {
    match scope {
        Scope::Library => app.ui.grid = value,
        Scope::Following => app.ui.following_grid = value,
    }
}

fn render(
    app: &mut Jondo,
    window: &mut Window,
    cx: &mut Context<Jondo>,
    scope: Scope,
) -> impl IntoElement {
    let narrow = window.viewport_size().width < theme::unit(768.0);
    let filter = filter(app, scope);
    let grid = grid(app, scope);
    let items = match scope {
        Scope::Library => Items::Library(app.shelf(cx)),
        Scope::Following => Items::Following(app.following_page(cx)),
    };

    let loading = {
        let library = app.library.read(cx);
        match scope {
            Scope::Library => {
                app.settings.read(cx).spotify == SpotifyState::Connecting
                    || (library.load.loading && matches!(filter, Filter::All | Filter::Spotify))
            }
            Scope::Following => library.following.loading && items.is_empty(),
        }
    };

    let panel = div()
        .size_full()
        .flex()
        .flex_col()
        .rounded(theme::unit(6.0))
        .bg(theme::SURFACE)
        .border_1()
        .border_color(theme::BORDER)
        .child(controls(app, cx, narrow, scope));

    if items.is_empty() {
        let body = if loading {
            loading::spinner().into_any_element()
        } else {
            empty(app, cx, scope)
        };

        return panel.child(div().flex_1().min_h(theme::unit(0.0)).child(body));
    }

    let cell = grid::cell(window);
    let scroll = app.scroll(match scope {
        Scope::Library => "shelf",
        Scope::Following => "following",
    });
    let gap = grid::gap(window);
    let columns = if grid { 3 } else { 1 };
    let row_height = if grid {
        cover::tile_height(cell) + gap
    } else {
        52.0
    };
    let count = items.len();
    let total_rows = count.div_ceil(columns);
    let entity = cx.entity();

    let rows = uniform_list("shelf-scroll", total_rows, move |range, _window, cx| {
        range
            .filter_map(|row| {
                let start = row * columns;
                let end = (start + columns).min(count);
                if start >= end {
                    return None;
                }

                Some(entity.update(cx, |app, cx| {
                    let mut cards = Vec::with_capacity(end - start);
                    for index in start..end {
                        let card = match (&items, grid) {
                            (Items::Library(items), true) => items.get(index).map(|item| {
                                card::tile(app, cx, item, cell, true, true).into_any_element()
                            }),
                            (Items::Library(items), false) => items.get(index).map(|item| {
                                row::show(app, cx, item, true, true).into_any_element()
                            }),
                            (Items::Following(items), true) => items.get(index).map(|item| {
                                follow_tile(app, cx, item, cell, true).into_any_element()
                            }),
                            (Items::Following(items), false) => items
                                .get(index)
                                .map(|item| follow_row(app, cx, item, true).into_any_element()),
                        };
                        if let Some(card) = card {
                            cards.push(card);
                        }
                    }

                    if grid {
                        div()
                            .flex()
                            .items_start()
                            .gap(theme::unit(gap))
                            .h(theme::unit(row_height))
                            .children(cards)
                            .into_any_element()
                    } else {
                        cards
                            .into_iter()
                            .next()
                            .unwrap_or_else(|| div().h(theme::unit(row_height)).into_any_element())
                    }
                }))
            })
            .collect()
    })
    .track_scroll(&scroll)
    .pl(theme::unit(grid::LEFT))
    .pr(theme::unit(grid::GUTTER))
    .pb(theme::unit(12.0))
    .h_full();

    panel.child(
        div()
            .debug_selector(|| "shelf-rows".to_owned())
            .relative()
            .flex_1()
            .h_full()
            .min_h(theme::unit(0.0))
            .child(rows)
            .child(scroll::bar(&scroll)),
    )
}
