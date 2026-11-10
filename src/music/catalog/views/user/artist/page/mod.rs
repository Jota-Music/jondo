use std::rc::Rc;
use std::time::Instant;

use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, Context, Div, ElementId, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Window, div, uniform_list,
};

use crate::music::track::views::{actions, list, row};
use crate::shared::i18n::{self, t};
use crate::shared::music::domain::{Album, Follow, FollowKind};
use crate::shared::music::views::cover;
use crate::shared::ui::views::components::paint::Scope;
use crate::shared::ui::views::components::{
    grid, hint, hold, hover, loading, reveal, scroll, text,
};
use crate::shared::ui::views::design::icons;
use crate::shared::ui::views::design::theme;
use crate::shell::app::{Jondo, Route};
use crate::shell::views::menu::{Command, Item, Menu};
use crate::shell::views::page::{self, Header, Play};

use super::tabs::tab_button;

fn group_of(album: &Album) -> &str {
    if album.group.trim().is_empty() {
        "album"
    } else {
        album.group.trim()
    }
}

fn groups_of(albums: &[Album]) -> Vec<String> {
    crate::shared::music::domain::groups(albums)
}

fn count_of(albums: &[Album], group: &str) -> usize {
    albums
        .iter()
        .filter(|album| group_of(album).eq_ignore_ascii_case(group))
        .count()
}

fn selected(app: &Jondo, groups: &[String]) -> String {
    let tab = app.ui.artist_tab.clone();
    if tab == "tracks" || groups.iter().any(|group| group == &tab) {
        tab
    } else {
        "tracks".to_owned()
    }
}

fn label(group: &str) -> &str {
    match group {
        "album" => t("Albums"),
        "single" => t("Singles"),
        "compilation" => t("Compilations"),
        "appears_on" => t("Appears on"),
        other => other,
    }
}

/// The tab grid Jota draws: two columns below 640px, six above it, where the
/// top-tracks tab and the first discography tab share the opening row.
fn tab_widths(width: f32) -> (f32, f32, f32, bool) {
    let padded = width < 768.0;
    let column = (width - if padded { 32.0 } else { 0.0 }).min(theme::CONTENT_WIDTH);
    let gap = 8.0;
    let stacked = width < 640.0;
    let cell = ((column - gap * 5.0) / 6.0).floor();
    let pair = if stacked {
        (column - gap) * 0.5
    } else {
        cell * 3.0 + gap * 2.0
    };

    let tail = if stacked {
        (column - gap) * 0.5
    } else {
        cell * 2.0 + gap
    };

    (column, pair, tail, stacked)
}

fn tabs(
    app: &Jondo,
    cx: &mut Context<Jondo>,
    window: &Window,
    id: &str,
    albums: &[Album],
    groups: &[String],
    selected: &str,
) -> Div {
    let (column, pair, tail, stacked) = tab_widths(window.viewport_size().width.as_f32());
    let gap = 8.0;
    let rest: Vec<AnyElement> = groups
        .iter()
        .enumerate()
        .map(|(position, group)| {
            let loading = !app.discography_ready(id, group, cx);
            let count = count_of(albums, group);
            tab_button(
                cx,
                group,
                label(group),
                selected == *group,
                loading,
                icons::DISC_3,
                (!loading).then_some(count),
                if !stacked && position == 0 {
                    pair
                } else {
                    tail
                },
            )
            .into_any_element()
        })
        .collect();
    let mut head = div().flex().gap(theme::unit(gap)).child(tab_button(
        cx,
        "tracks",
        t("Top tracks"),
        selected == "tracks",
        false,
        icons::LIST_MUSIC,
        None,
        if stacked { column } else { pair },
    ));
    let mut rest = rest;
    if !stacked && !rest.is_empty() {
        head = head.child(rest.remove(0));
    }

    let per_row = if stacked { 2 } else { 3 };
    let mut layout = div().flex().flex_col().gap(theme::unit(gap)).child(head);
    while !rest.is_empty() {
        let take = per_row.min(rest.len());
        let mut row = div().flex().gap(theme::unit(gap));
        for tab in rest.drain(0..take) {
            row = row.child(tab);
        }

        layout = layout.child(row);
    }

    layout
}

/// The artist before its data lands: the header keeps its frame, and the
/// content stays on the same block loader the rest of the app uses. A section
/// whose data is still on its way stays disabled until it can be opened.
fn skeleton(app: &mut Jondo, window: &mut Window, cx: &mut Context<Jondo>, id: &str) -> AnyElement {
    let groups = groups_of(&[]);
    let selected = selected(app, &groups);
    let header = page::pending(64.0, 6.0).child(
        div()
            .w(theme::unit(32.0))
            .h(theme::unit(32.0))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .rounded(theme::unit(6.0))
            .border_1()
            .border_color(theme::ZINC_800)
            .bg(theme::SURFACE)
            .child(icons::icon(icons::HEART, 14.0, theme::ZINC_700)),
    );
    let tabs = tabs(app, cx, window, id, &[], &groups, &selected);
    let content = if selected == "tracks" {
        loading::rows()
    } else {
        page::panel(loading::spinner())
    };

    div()
        .size_full()
        .flex()
        .flex_col()
        .gap(theme::unit(16.0))
        .child(header)
        .child(tabs)
        .child(content)
        .into_any_element()
}

pub fn show(app: &mut Jondo, window: &mut Window, cx: &mut Context<Jondo>) -> AnyElement {
    let Route::Artist(id) = app.route.clone() else {
        return div().into_any_element();
    };

    let Some(artist) = app.artist(&id, cx) else {
        if app.catalog.read(cx).artists.loading(&id) {
            return skeleton(app, window, cx, &id);
        }

        return hint::plain(t("Artist not found")).into_any_element();
    };

    let artist_id = artist.id.clone();
    let artist_name = artist.name.clone();
    let artist_image = artist.image.clone();
    let followers = artist.followers;
    let albums = artist.albums.clone();
    let groups = groups_of(&albums);
    let table = app.artist_rows(&id, cx);
    let tracks = table.full.clone();
    let subtitle =
        (followers > 0).then(|| i18n::count(followers as usize, "1 follower", "{count} followers"));

    let selected = selected(app, &groups);
    let selected = if selected != "tracks"
        && app.discography_ready(&id, &selected, cx)
        && count_of(&albums, &selected) == 0
    {
        "tracks".to_owned()
    } else {
        selected
    };

    let summary = Follow::spotify(
        FollowKind::Artist,
        &artist_id,
        &artist_name,
        artist_image.clone(),
    );
    let menu_summary = summary.clone();
    let current = app.player.read(cx).current().map(|song| song.id.clone());
    let in_list = selected == "tracks"
        && current
            .as_deref()
            .is_some_and(|id| tracks.iter().any(|song| song.id == id));
    let header = page::show(
        app,
        cx,
        Header {
            cover: artist_image.as_deref(),
            covers: &[],
            radius: 6.0,
            title: &artist_name,
            subtitle,
            link: None,
            save: None,
            follow: Some(summary),
            play: Some(Play {
                enabled: selected == "tracks" && !tracks.is_empty(),
                playing: in_list && app.player.read(cx).playing,
                loading: in_list && app.player.read(cx).loading,
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
            actions: (selected == "tracks" && !tracks.is_empty()).then(|| {
                let tracks = tracks.clone();
                let id = artist_id.clone();
                let summary = menu_summary.clone();
                Box::new(move |app: &Jondo, cx: &gpui::App| {
                    let followed = app.following().iter().any(|item| item.id == summary.id);
                    let lead = vec![Item::new(
                        if followed {
                            icons::HEART_FILL
                        } else {
                            icons::HEART
                        },
                        if followed { t("Unfollow") } else { t("Follow") },
                        Command::Follow(summary.clone()),
                    )];
                    actions::page(app, cx, (*tracks).clone(), "artist", &id, lead)
                }) as Box<dyn Fn(&Jondo, &gpui::App) -> Vec<Item>>
            }),
        },
    );

    let albums = Rc::new(albums);
    let tabs = tabs(app, cx, window, &id, albums.as_slice(), &groups, &selected);
    let content = content(app, cx, window, &id, albums, &selected);

    div()
        .size_full()
        .flex()
        .flex_col()
        .gap(theme::unit(16.0))
        .child(header)
        .child(tabs)
        .child(content)
        .into_any_element()
}

fn content(
    app: &mut Jondo,
    cx: &mut Context<Jondo>,
    window: &Window,
    id: &str,
    albums: Rc<Vec<Album>>,
    selected: &str,
) -> AnyElement {
    if selected == "tracks" {
        let table = app.artist_rows(id, cx);
        return page::panel(list::show(
            app,
            cx,
            list::List {
                key: format!("artist:{id}"),
                id: "artist-tracks",
                table,
                playlist: None,
                removable: false,
                shape: row::Shape::Song,
                focus: true,
            },
        ));
    }

    if !app.discography_ready(id, selected, cx) {
        return page::panel(loading::spinner());
    }

    let filtered: Rc<Vec<usize>> = Rc::new(
        albums
            .iter()
            .enumerate()
            .filter(|(_, album)| group_of(album).eq_ignore_ascii_case(selected))
            .map(|(index, _)| index)
            .collect(),
    );
    let cell = grid::cell(window);
    let grid = app.ui.artist_grid;
    let scroll = app.scroll(format!("artist-albums:{id}"));
    let gap = grid::gap(window);
    let columns = if grid { 3 } else { 1 };
    let row_height = if grid {
        cover::tile_height(cell) + gap
    } else {
        52.0
    };
    let count = filtered.len();
    let total_rows = count.div_ceil(columns);
    let entity = cx.entity();

    let rows = uniform_list("artist-albums", total_rows, move |range, _window, cx| {
        range
            .filter_map(|slot| {
                let start = slot * columns;
                let end = (start + columns).min(count);
                if start >= end {
                    return None;
                }

                Some(entity.update(cx, |app, cx| {
                    let mut cards = Vec::with_capacity(end - start);
                    for position in start..end {
                        let Some(index) = filtered.get(position) else {
                            continue;
                        };
                        let Some(album) = albums.get(*index) else {
                            continue;
                        };

                        cards.push(if grid {
                            tile(app, cx, album, cell, true)
                        } else {
                            list_row(app, cx, album, true)
                        });
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

    let body = div()
        .relative()
        .flex_1()
        .h_full()
        .min_h(theme::unit(0.0))
        .child(rows)
        .child(scroll::bar(&scroll))
        .into_any_element();

    let toggle = div()
        .flex()
        .justify_end()
        .px(theme::unit(6.0))
        .pt(theme::unit(6.0))
        .child(grid::toggle(
            "artist-view",
            grid,
            32.0,
            cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                app.ui.artist_grid = true;
                cx.notify();
            }),
            cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                app.ui.artist_grid = false;
                cx.notify();
            }),
        ));
    page::panel(
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(toggle)
            .child(body),
    )
}

fn open_menu(
    app: &mut Jondo,
    cx: &mut Context<Jondo>,
    position: gpui::Point<gpui::Pixels>,
    album: &Album,
) {
    let items = actions::album_card(app, album, cx);
    if items.is_empty() {
        return;
    }

    app.menu = Some(Menu::context(position, None, items, 0.0));
    app.notify_shell(cx);
}

fn tile(
    app: &mut Jondo,
    cx: &mut Context<Jondo>,
    album: &Album,
    cell: f32,
    shown: bool,
) -> AnyElement {
    let art = if shown {
        cover::art(
            &mut app.covers,
            album.cover.as_deref().unwrap_or_default(),
            cell - 12.0,
            cell - 12.0,
            6.0,
        )
    } else {
        cover::placeholder(cell - 12.0, cell - 12.0, 6.0)
    };

    let album_id = album.id.clone();
    let play_id = album.id.clone();
    let group = format!("artist-album-{}", album.id);
    let menu = album.clone();
    let hold_album = album.clone();
    let entity = cx.entity();
    let play = div()
        .absolute()
        .right(theme::unit(10.0))
        .top(theme::unit(10.0))
        .child(reveal::show(
            &group,
            reveal::button(
                format!("artist-album-play-{}", album.id),
                icons::PLAY,
                16.0,
                theme::rgb(0xffffff),
                Some(theme::black(0.8)),
                6.0,
                cx.listener(move |app, _: &gpui::ClickEvent, _, cx| {
                    cx.stop_propagation();
                    app.play_album(&play_id, cx);
                    cx.notify();
                }),
            ),
        ));
    div()
        .id(format!("artist-album-{}", album.id))
        .group(group)
        .w(theme::unit(cell))
        .h(theme::unit(cover::tile_height(cell)))
        .flex_none()
        .flex()
        .flex_col()
        .cursor_pointer()
        .on_mouse_down(
            gpui::MouseButton::Right,
            cx.listener(move |app, event: &gpui::MouseDownEvent, _, cx| {
                open_menu(app, cx, event.position, &menu);
            }),
        )
        .on_click(cx.listener(move |app, _: &gpui::ClickEvent, _, cx| {
            app.open(Route::Album(album_id.clone()), cx);
            cx.notify();
        }))
        .child(hold::hold(move |position, _, cx| {
            entity.update(cx, |app, cx| {
                open_menu(app, cx, position, &hold_album);
            });
        }))
        .child(
            div()
                .relative()
                .px(theme::unit(6.0))
                .pt(theme::unit(6.0))
                .child(art)
                .child(play),
        )
        .child(
            div()
                .px(theme::unit(6.0))
                .pt(theme::unit(8.0))
                .pb(theme::unit(6.0))
                .flex()
                .flex_col()
                .gap(theme::unit(2.0))
                .child(text::truncated(
                    album.name.clone(),
                    14.0,
                    theme::TEXT,
                    theme::Weight::Medium,
                ))
                .child(text::truncated(
                    year(album),
                    12.0,
                    theme::FAINT,
                    theme::Weight::Regular,
                )),
        )
        .into_any_element()
}

fn list_row(app: &mut Jondo, cx: &mut Context<Jondo>, album: &Album, shown: bool) -> AnyElement {
    let group = format!("artist-row-{}", album.id);
    let id = ElementId::Name(group.clone().into());
    let fade = hover::value(&id, Instant::now());
    let art = if shown {
        cover::art(
            &mut app.covers,
            album.cover.as_deref().unwrap_or_default(),
            40.0,
            40.0,
            6.0,
        )
    } else {
        cover::placeholder(40.0, 40.0, 6.0)
    };

    let album_id = album.id.clone();
    let play_id = album.id.clone();
    let menu = album.clone();
    let hold_album = album.clone();
    let entity = cx.entity();
    let play = reveal::show(
        &group,
        reveal::button(
            format!("artist-row-play-{}", album.id),
            icons::PLAY,
            16.0,
            theme::rgb(0xffffff),
            Some(theme::black(0.8)),
            6.0,
            cx.listener(move |app, _: &gpui::ClickEvent, _, cx| {
                cx.stop_propagation();
                app.play_album(&play_id, cx);
                cx.notify();
            }),
        ),
    );
    div()
        .id(id.clone())
        .group(group)
        .w_full()
        .h(theme::unit(52.0))
        .flex()
        .items_center()
        .rounded(theme::unit(6.0))
        .cursor_pointer()
        .when(fade > 0.0, |this| {
            this.bg(theme::alpha(theme::ZINC_900, fade))
        })
        .on_hover(cx.listener({
            let id = id.clone();
            move |app, on: &bool, window, cx| {
                hover::set_in(
                    Scope::Page,
                    &id,
                    *on,
                    Instant::now(),
                    window.last_input_was_touch(),
                );
                app.paint_page(cx);
            }
        }))
        .on_mouse_down(
            gpui::MouseButton::Right,
            cx.listener(move |app, event: &gpui::MouseDownEvent, _, cx| {
                open_menu(app, cx, event.position, &menu);
            }),
        )
        .on_click(cx.listener(move |app, _: &gpui::ClickEvent, _, cx| {
            app.open(Route::Album(album_id.clone()), cx);
            cx.notify();
        }))
        .child(hold::hold(move |position, _, cx| {
            entity.update(cx, |app, cx| {
                open_menu(app, cx, position, &hold_album);
            });
        }))
        .child(div().w(theme::unit(8.0)))
        .child(art)
        .child(div().w(theme::unit(12.0)))
        .child(
            div()
                .flex_1()
                .min_w(theme::unit(0.0))
                .flex()
                .flex_col()
                .gap(theme::unit(2.0))
                .child(text::truncated(
                    album.name.clone(),
                    14.0,
                    theme::TEXT,
                    theme::Weight::Medium,
                ))
                .child(text::truncated(
                    year(album),
                    12.0,
                    theme::FAINT,
                    theme::Weight::Regular,
                )),
        )
        .child(play)
        .child(div().w(theme::unit(8.0)))
        .into_any_element()
}

fn year(album: &Album) -> String {
    if album.year > 0 {
        album.year.to_string()
    } else {
        String::new()
    }
}

#[cfg(test)]
#[path = "page.test.rs"]
mod tests;
