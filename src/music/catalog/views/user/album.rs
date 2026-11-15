use crate::shared::ui::views::design::theme;

use gpui::{AnyElement, Context, IntoElement, ParentElement, Styled, div};

use crate::music::track::views::{actions, list, row};
use crate::shared::i18n::{self, t};
use crate::shared::ui::views::components::{hint, loading};
use crate::shell::app::{Jondo, Route};
use crate::shell::views::page::{self, Header, Play};

pub fn show(app: &mut Jondo, cx: &mut Context<Jondo>) -> AnyElement {
    let Route::Album(id) = app.route.clone() else {
        return div().into_any_element();
    };

    let Some(album) = app.album(&id, cx) else {
        if app.catalog.read(cx).albums.loading(&id) {
            return skeleton();
        }

        return hint::plain(t("Album not found")).into_any_element();
    };

    let album_id = album.id.clone();
    let album_name = album.name.clone();
    let album_cover = album.cover.clone();
    let album_year = album.year;
    let album_artists = album.artists.clone();
    let table = app.album_rows(&id, cx);
    let tracks = table.full.clone();
    let count = tracks.len();
    let subtitle = {
        let mut parts = Vec::new();
        if album_year > 0 {
            parts.push(album_year.to_string());
        }

        parts.push(i18n::count(count, "1 track", "{count} tracks"));
        Some(parts.join(" · "))
    };

    let link = album_artists
        .first()
        .filter(|artist| !artist.id.is_empty())
        .map(|artist| (artist.name.clone(), Route::Artist(artist.id.clone())));
    let current = app.player.read(cx).current().map(|song| song.id.clone());
    let in_list = current
        .as_deref()
        .is_some_and(|id| tracks.iter().any(|song| song.id == id));
    let header = page::show(
        app,
        cx,
        Header {
            cover: album_cover.as_deref(),
            covers: &[],
            radius: 6.0,
            title: &album_name,
            subtitle,
            link,
            save: None,
            follow: None,
            play: Some(Play {
                enabled: !tracks.is_empty(),
                playing: in_list && app.player.read(cx).playing,
                paused: in_list && app.player.read(cx).paused,
                loading: in_list && app.player.read(cx).busy(),
                on_click: Box::new(cx.listener({
                    let tracks = tracks.clone();
                    move |app, _: &gpui::ClickEvent, _, cx| {
                        if !tracks.is_empty() {
                            app.toggle_list(&tracks, cx);
                            cx.notify();
                        }
                    }
                })),
            }),
            extra: None,
            actions: (!tracks.is_empty()).then(|| {
                Box::new({
                    let tracks = tracks.clone();
                    let id = album_id.clone();
                    move |app: &Jondo, cx: &gpui::App| {
                        actions::page(app, cx, (*tracks).clone(), "album", &id, Vec::new())
                    }
                })
                    as Box<dyn Fn(&Jondo, &gpui::App) -> Vec<crate::shell::views::menu::Item>>
            }),
        },
    );
    let list = page::panel(list::show(
        app,
        cx,
        list::List {
            key: format!("album:{id}"),
            id: "album-tracks",
            table: table.clone(),
            playlist: None,
            removable: false,
            shape: row::Shape::Song,
            focus: true,
        },
    ));
    div()
        .size_full()
        .flex()
        .flex_col()
        .gap(theme::unit(16.0))
        .child(header)
        .child(list)
        .into_any_element()
}

/// The disc before its data lands: the header keeps the frame the real one will
/// fill and the tracklist keeps its shell, so opening an album never blanks the
/// page to a spinner in the middle of the scenery.
fn skeleton() -> AnyElement {
    div()
        .size_full()
        .flex()
        .flex_col()
        .gap(theme::unit(16.0))
        .child(page::pending(64.0, 6.0))
        .child(loading::rows())
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use gpui::{TestAppContext, VisualTestContext};

    use crate::shared::music::domain::{Album, Song};
    use crate::shared::storage::Store;
    use crate::shared::ui::views::design::theme;
    use crate::shell::app::{Jondo, Route};

    fn home() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("jondo-album-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        crate::shared::storage::use_home(&dir);
        dir
    }

    fn frame(cx: &mut VisualTestContext) {
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
        });
        cx.run_until_parked();
    }

    /// Reopening a disc paints the copy already in memory: the route no longer
    /// drops the album before fetching it, so the page keeps its header and its
    /// tracklist instead of blanking to a skeleton.
    #[gpui::test]
    fn a_cached_album_opens_on_its_tracks(cx: &mut TestAppContext) {
        home();
        let store = Store::load();

        cx.update(|cx| {
            gpui_kit::init(cx);
            theme::install(cx);
        });

        let (app, cx) = cx.add_window_view(|window, cx| Jondo::boot(store, window, cx));
        cx.run_until_parked();
        frame(cx);

        app.update(cx, |app, cx| {
            app.catalog.update(cx, |catalog, _| {
                catalog.albums.ready(
                    "spotify:album:one".to_owned(),
                    Album {
                        id: "spotify:album:one".to_owned(),
                        name: "One".to_owned(),
                        tracks: vec![Song {
                            id: "spotify:track:one".to_owned(),
                            name: "Track".to_owned(),
                            duration: 180,
                            ..Default::default()
                        }],
                        ..Default::default()
                    },
                );
            });
            app.open(Route::Album("spotify:album:one".to_owned()), cx);
            cx.notify();
        });
        cx.run_until_parked();
        frame(cx);
        frame(cx);

        assert!(
            cx.debug_bounds("page-play").is_some(),
            "the cached disc must paint its header instead of a skeleton"
        );
    }
}
