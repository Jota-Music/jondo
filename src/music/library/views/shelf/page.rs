use std::rc::Rc;

use gpui::{AnyElement, Context, IntoElement, ParentElement, Styled, Window, div};

use crate::music::library::views::playlist::{card, row};
use crate::shared::music::domain::{Follow, Playlist};
use crate::shared::ui::views::components::{grid, loading};
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
    if scope == Scope::Following {
        app.ensure_follow_meta(cx);
    }

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
    let scroll = app.pane(match scope {
        Scope::Library => "shelf",
        Scope::Following => "following",
    });
    let columns = if grid { 3 } else { 1 };
    let row_height = if grid {
        cell + grid::gap(window) + 44.0
    } else {
        52.0
    };

    let shown = grid::visible(
        &scroll,
        window.viewport_size().height,
        row_height,
        columns,
        items.len(),
    );

    let rows: Vec<AnyElement> = match (&items, grid) {
        (Items::Library(items), true) => items
            .iter()
            .enumerate()
            .map(|(ix, item)| {
                card::tile(app, cx, item, cell, true, shown.contains(&ix)).into_any_element()
            })
            .collect(),
        (Items::Library(items), false) => items
            .iter()
            .enumerate()
            .map(|(ix, item)| {
                row::show(app, cx, item, true, shown.contains(&ix)).into_any_element()
            })
            .collect(),
        (Items::Following(items), true) => items
            .iter()
            .enumerate()
            .map(|(ix, item)| {
                follow_tile(app, cx, item, cell, shown.contains(&ix)).into_any_element()
            })
            .collect(),
        (Items::Following(items), false) => items
            .iter()
            .enumerate()
            .map(|(ix, item)| follow_row(app, cx, item, shown.contains(&ix)).into_any_element())
            .collect(),
    };

    let content = if grid {
        grid::show("shelf-scroll", &scroll, window, rows)
    } else {
        grid::list("shelf-scroll", &scroll, rows)
    };

    panel.child(content)
}
