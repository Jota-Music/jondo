use std::rc::Rc;

use gpui::{AnyElement, Context, IntoElement, ParentElement, Stateful, Styled, div};

use crate::shared::i18n;
use crate::shared::music::domain::Song;
use crate::shared::ui::views::components::paint::Scope;
use crate::shared::ui::views::components::{banner, button, text};
use crate::shared::ui::views::design::icons;
use crate::shared::ui::views::design::theme;
use crate::shell::app::{Jondo, Route, SearchType};
use crate::shell::views::menu::Command;

use super::actions;

/// The songs the current page holds and whether its rows can be removed from
/// it. A selection keeps only ids, so the page under the banner is what turns
/// them back into the songs the actions play, enqueue or pick.
fn current(app: &mut Jondo, cx: &gpui::App) -> (Rc<Vec<Song>>, bool) {
    if app.ui.queue_open {
        return (Rc::new(Vec::new()), false);
    }

    match app.route.clone() {
        Route::Playlist(id) => {
            let removable = app
                .view(&id, cx)
                .is_some_and(|playlist| playlist.is_local());
            (app.rows(&id, cx).full, removable)
        }
        Route::Album(id) => (app.album_rows(&id, cx).full, false),
        Route::Artist(id) => (app.artist_rows(&id, cx).full, false),
        Route::Channel(id) if app.ui.channel_tab == "videos" => {
            (app.channel_rows(&id, cx).full, false)
        }
        Route::Catalog(SearchType::Track, _) => (app.catalog_rows(cx).full, false),
        Route::Search(_) if app.search.tab == "videos" => (app.search_rows().full, false),
        _ => (Rc::new(Vec::new()), false),
    }
}

/// The banner under the header, next to the clipboard one. It belongs to the
/// layout so every song list gets it without each page wiring its own.
pub fn bar(app: &mut Jondo, cx: &mut Context<Jondo>) -> AnyElement {
    let (songs, removable) = current(app, cx);
    let selected: Vec<Song> = songs
        .iter()
        .filter(|song| app.selection.iter().any(|id| id == &song.id))
        .cloned()
        .collect();
    if selected.len() <= 1 {
        return div().into_any_element();
    }

    let playlist = match &app.route {
        Route::Playlist(id) if removable => Some(id.clone()),
        _ => None,
    };

    let targets = selected.clone();

    banner::banner()
        .child(
            div()
                .w(theme::unit(28.0))
                .h(theme::unit(28.0))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .rounded_full()
                .bg(app.accent)
                .child(icons::icon(icons::CHECK_BOLD, 14.0, app.binary)),
        )
        .child(text::text(
            i18n::count(selected.len(), "1 selected", "{count} selected"),
            14.0,
            theme::MUTED,
        ))
        .child(div().flex_1())
        .child(
            div()
                .flex()
                .items_center()
                .gap(theme::unit(4.0))
                .child(actions::trigger_in(
                    Scope::Shell,
                    cx,
                    "selection-actions",
                    28.0,
                    move |app, cx| {
                        let ids: Vec<String> = targets.iter().map(|song| song.id.clone()).collect();
                        let remove = playlist.clone().map(|id| Command::Remove(id, ids));
                        let items = actions::track(
                            app,
                            cx,
                            Vec::new(),
                            targets.clone(),
                            Command::Play(targets.clone(), 0),
                            None,
                            Some(targets.clone()),
                            remove,
                            Vec::new(),
                        );
                        (None, items)
                    },
                    0.0,
                )),
        )
        .child(clear(cx))
        .into_any_element()
}

fn clear(cx: &mut Context<Jondo>) -> Stateful<gpui::Div> {
    button::dismiss_in(
        Scope::Shell,
        "selection-clear",
        cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
            app.clear_selection();
            cx.notify();
        }),
    )
}
