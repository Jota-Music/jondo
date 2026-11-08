use gpui::{AnyElement, Context, IntoElement, ParentElement, Styled, Window, div};
use gpui_kit::component::input::InputEvent;

use crate::music::track::views::{actions, list, row as track_row};
use crate::shared::i18n::{self, t};
use crate::shared::music::domain::{Playlist, Source};
use crate::shared::ui::views::components::{
    grid,
    hint::{self, Hint},
    loading,
};

use crate::shared::ui::views::design::icons;
use crate::shared::ui::views::design::theme;
use crate::shell::app::{Jondo, Route};
use crate::shell::views::menu;
use crate::shell::views::page::{self, Header, Play};
use crate::shell::views::toolbar::{self, Toolbar};

use super::{card, row};

/// Liked Songs arrives from Spotify with no name of its own.
pub(crate) fn title(playlist: &Playlist) -> String {
    if playlist.liked() {
        t("Liked Songs").to_owned()
    } else {
        playlist.name.clone()
    }
}

fn bar(app: &mut Jondo, window: &mut Window, cx: &mut Context<Jondo>) -> AnyElement {
    let id = match app.route.clone() {
        Route::Playlist(id) => id,
        _ => String::new(),
    };

    let query = app.ui.playlist_query.clone();
    let refreshing = app.catalog.read(cx).playlists.loading(&id);
    toolbar::show(
        app,
        window,
        cx,
        Toolbar {
            id: "playlist",
            field: "playlist-search",
            placeholder: t("Search in playlist"),
            query,
            refreshing,
            origin: "playlist-order",
            order: app.ui.playlist_order.label().to_owned(),
            items: menu::playlist_order(app.ui.playlist_order),
            clear: Box::new(cx.listener(|app, _: &gpui::ClickEvent, window, cx| {
                app.ui.playlist_query.clear();
                app.set_field("playlist-search", "", window, cx);
                cx.notify();
            })),
            reload: Box::new(cx.listener(move |app, _: &gpui::ClickEvent, _, cx| {
                app.reload_playlist(&id, cx);
                cx.notify();
            })),
        },
    )
    .into_any_element()
}

/// A playlist collection in either of the two shapes the library and the search
/// results already use: cards or rows. The caller owns the toolbar and the panel.
pub fn results(
    app: &mut Jondo,
    window: &Window,
    cx: &mut Context<Jondo>,
    key: impl Into<String>,
    id: &'static str,
    playlists: &[Playlist],
    grid: bool,
) -> AnyElement {
    let scroll = app.pane(key);
    if grid {
        let cell = grid::cell(window);
        let shown = grid::visible(
            &scroll,
            window.viewport_size().height,
            cell + grid::gap(window) + 44.0,
            3,
            playlists.len(),
        );
        let tiles: Vec<AnyElement> = playlists
            .iter()
            .enumerate()
            .map(|(ix, playlist)| {
                card::tile(app, cx, playlist, cell, false, shown.contains(&ix)).into_any_element()
            })
            .collect();
        return grid::show(id, &scroll, window, tiles);
    }

    let shown = grid::visible(
        &scroll,
        window.viewport_size().height,
        52.0,
        1,
        playlists.len(),
    );
    let rows: Vec<AnyElement> = playlists
        .iter()
        .enumerate()
        .map(|(ix, playlist)| {
            row::show(app, cx, playlist, false, shown.contains(&ix)).into_any_element()
        })
        .collect();
    grid::list(id, &scroll, rows)
}

pub fn show(app: &mut Jondo, window: &mut Window, cx: &mut Context<Jondo>) -> AnyElement {
    let Route::Playlist(id) = app.route.clone() else {
        return div().into_any_element();
    };

    let Some(playlist) = app.view(&id, cx) else {
        if app.catalog.read(cx).playlists.loading(&id) {
            return skeleton(app, window, cx);
        }

        return missing(cx, &id);
    };

    let empty = app.songs(&playlist.id, cx).is_empty();
    if playlist.loading && empty {
        return skeleton(app, window, cx);
    }

    let here = app.player.read(cx).current().is_some_and(|song| {
        app.songs(&playlist.id, cx)
            .iter()
            .any(|item| item.id == song.id)
    });
    let count = playlist.track_count;
    let tracks = i18n::count(count as usize, "1 track", "{count} tracks");

    let title = title(&playlist);
    let link = playlist
        .owner
        .clone()
        .filter(|owner| !owner.is_empty())
        .and_then(|owner| {
            let route = match playlist.source {
                Source::Spotify => Route::User(owner.clone()),
                Source::YouTube => Route::Channel(playlist.owner_id.clone()?),
                Source::Local => return None,
            };

            Some((owner, route))
        });
    let save = (!playlist.is_local() && !playlist.liked()).then(|| playlist.clone());
    let menu_id = playlist.id.clone();
    let play_id = playlist.id.clone();
    let header = page::show(
        app,
        cx,
        Header {
            cover: playlist.cover.as_deref(),
            covers: &playlist.covers,
            radius: 6.0,
            title: &title,
            subtitle: Some(tracks),
            link,
            save,
            follow: None,
            play: Some(Play {
                enabled: !empty,
                playing: app.player.read(cx).playing && here,
                loading: app.player.read(cx).loading && here,
                on_click: Box::new(cx.listener(move |app, _: &gpui::ClickEvent, _, cx| {
                    app.play_playlist(&play_id, cx);
                    cx.notify();
                })),
            }),
            extra: None,
            actions: Some(Box::new(move |app: &Jondo, cx: &gpui::App| {
                app.view(&menu_id, cx)
                    .map(|playlist| actions::playlist(app, &playlist, cx))
                    .unwrap_or_default()
            })),
        },
    );

    let table = app.rows(&playlist.id, cx);

    let spinner = playlist.loading && table.full.is_empty();
    let body = if spinner {
        loading::rows()
    } else if table.full.is_empty() {
        page::panel(hint::muted(t("No songs")))
    } else {
        page::panel(list::show(
            app,
            cx,
            list::List {
                key: format!("playlist:{}", playlist.id),
                id: "playlist-scroll",
                table: table.clone(),
                playlist: Some(playlist.id.clone()),
                removable: playlist.is_local(),
                shape: track_row::Shape::Song,
                focus: true,
            },
        ))
    };

    let controls = bar(app, window, cx);

    div()
        .size_full()
        .flex()
        .flex_col()
        .gap(theme::unit(16.0))
        .child(header)
        .child(controls)
        .child(body)
        .into_any_element()
}

/// The page before its data lands: the header boxes shimmer, the toolbar is
/// already in place (its reload button spins where it will always sit) and the
/// list keeps the frame the rows will fill.
fn skeleton(app: &mut Jondo, window: &mut Window, cx: &mut Context<Jondo>) -> AnyElement {
    div()
        .size_full()
        .flex()
        .flex_col()
        .gap(theme::unit(16.0))
        .child(page::pending(64.0, 6.0))
        .child(bar(app, window, cx))
        .child(loading::rows())
        .into_any_element()
}

fn missing(cx: &mut Context<Jondo>, id: &str) -> AnyElement {
    let _ = id;
    hint::hint(
        Hint {
            icon: icons::CIRCLE_ALERT,
            tint: theme::AMBER,
            title: t("Playlist not found"),
            body: t("This playlist is no longer in your library."),
            steps: &[],
        },
        Some(hint::action(
            "missing-home",
            t("Back to playlists"),
            None,
            cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                app.open(Route::Home, cx);
                cx.notify();
            }),
        )),
    )
    .into_any_element()
}

pub fn wire(app: &mut Jondo, window: &mut Window, cx: &mut Context<Jondo>) {
    let field = app.field("playlist-search", "", t("Search in playlist"), window, cx);
    app.subscriptions.push(cx.subscribe_in(
        &field,
        window,
        |app, state, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::Change) {
                app.ui.playlist_query = state.read(cx).value().to_string();
                cx.notify();
            }
        },
    ));
}
