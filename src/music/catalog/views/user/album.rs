use crate::shared::ui::views::design::theme;

use gpui::{AnyElement, Context, IntoElement, ParentElement, Styled, div};

use crate::music::track::views::{actions, list, row};
use crate::shared::i18n::{self, t};
use crate::shared::ui::views::components::hint;
use crate::shell::app::{Jondo, Route};
use crate::shell::views::page::{self, Header, Play};

pub fn show(app: &mut Jondo, cx: &mut Context<Jondo>) -> AnyElement {
    let Route::Album(id) = app.route.clone() else {
        return div().into_any_element();
    };

    let Some(album) = app.album(&id, cx) else {
        if app.catalog.read(cx).albums.loading(&id) {
            return page::loading();
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
                loading: in_list && app.player.read(cx).loading,
                on_click: Box::new(cx.listener({
                    let tracks = tracks.clone();
                    move |app, _: &gpui::ClickEvent, _, cx| {
                        if !tracks.is_empty() {
                            app.play_list((*tracks).clone(), 0, cx);
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
