use gpui::prelude::FluentBuilder as _;
use gpui::{
    App, Context, Div, ElementId, InteractiveElement, ParentElement, SharedString, Stateful,
    StatefulInteractiveElement, Styled, div,
};

use gpui_kit::component::tooltip::Tooltip;

use crate::shared::i18n::t;
use crate::shared::music::domain::{Follow, Playlist, Song, Source};
use crate::shared::ui::views::components::hover;
use crate::shared::ui::views::components::paint::{self, Scope};
use crate::shared::ui::views::design::icons;
use crate::shared::ui::views::design::theme;
use crate::shell::app::{Jondo, Route};
use crate::shell::views::menu::{Command, Item, Menu};

#[allow(clippy::too_many_arguments)]
pub fn track(
    app: &Jondo,
    cx: &App,
    mut lead: Vec<Item>,
    targets: Vec<Song>,
    play: Command,
    toggle: Option<(String, usize)>,
    select_all: Option<Vec<Song>>,
    remove: Option<Command>,
    tail: Vec<Item>,
) -> Vec<Item> {
    let list: Vec<Song> = targets.into_iter().filter(|song| !song.broken).collect();
    if list.is_empty() {
        lead.extend(tail);
        return lead;
    }

    let single = list.len() == 1;
    let playing = single
        && app.player.read(cx).playing
        && app
            .player
            .read(cx)
            .current()
            .is_some_and(|current| current.id == list[0].id);

    let mut items = lead;
    items.push(Item::new(
        if playing {
            icons::PAUSE_LINE
        } else {
            icons::PLAY_LINE
        },
        if playing { t("Pause") } else { t("Play") },
        if playing { Command::Toggle } else { play },
    ));
    items.push(Item::new(
        icons::LIST_PLUS,
        t("Add to queue"),
        Command::Enqueue(list.clone()),
    ));
    items.push(Item::new(
        icons::SQUARE_PLUS,
        t("Add to playlist"),
        Command::Pick(list.clone()),
    ));

    if (select_all.is_some() || list.len() > 1) && list.len() > 1 {
        let all = list
            .iter()
            .all(|song| app.selection.iter().any(|id| id == &song.id));
        items.push(Item::new(
            if all {
                icons::SQUARE
            } else {
                icons::LIST_CHECKS
            },
            if all {
                t("Deselect all")
            } else {
                t("Select all")
            },
            if all {
                Command::Clear
            } else {
                Command::Select(list.clone())
            },
        ));
    }

    if let Some((id, index)) = toggle.filter(|_| single) {
        let selected = app.selection.iter().any(|item| item == &id);
        items.push(Item::new(
            if selected {
                icons::SQUARE
            } else {
                icons::SQUARE_CHECK
            },
            if selected { t("Deselect") } else { t("Select") },
            Command::ToggleOne(id, index),
        ));
    }

    if single {
        if list[0].source == Source::Spotify {
            items.push(Item::new(
                icons::RADIO,
                t("Go to radio"),
                Command::Open(Route::Playlist(radio_id(&list[0].id))),
            ));
        }

        items.push(Item::new(
            icons::LINK_2,
            t("Assign audio"),
            Command::Edit(list[0].clone()),
        ));
    }

    if let Some(remove) = remove {
        items.push(Item::danger(
            icons::TRASH_2,
            t("Remove from playlist"),
            remove,
        ));
    }

    if single {
        if let Some(link) = spotify_link(&list[0]) {
            items.push(Item::new(
                icons::Icon::Spotify,
                t("Share Spotify"),
                Command::Copy(link),
            ));
        }

        if let Some(link) = youtube_link(&list[0]) {
            items.push(Item::new(
                icons::Icon::YouTube,
                t("Share YouTube"),
                Command::Copy(link),
            ));
        }
    }

    items.extend(tail);
    items.sort_by_key(|item| item.danger);
    items
}

pub fn page(
    app: &Jondo,
    cx: &App,
    songs: Vec<Song>,
    page: &str,
    id: &str,
    lead: Vec<Item>,
) -> Vec<Item> {
    let playable: Vec<Song> = songs.iter().filter(|song| !song.broken).cloned().collect();
    if playable.is_empty() {
        return lead;
    }

    let tail = page_link(page, id)
        .map(|link| vec![Item::new(icons::LINK_2, t("Share"), Command::Copy(link))])
        .unwrap_or_default();
    track(
        app,
        cx,
        lead,
        playable.clone(),
        Command::Play(playable.clone(), 0),
        None,
        Some(playable),
        None,
        tail,
    )
}

pub fn channel(app: &Jondo, summary: &Follow, id: &str) -> Vec<Item> {
    let followed = app.following().iter().any(|item| item.id == summary.id);
    let mut items = vec![Item::new(
        if followed {
            icons::HEART_FILL
        } else {
            icons::HEART
        },
        if followed { t("Unfollow") } else { t("Follow") },
        Command::Follow(summary.clone()),
    )];
    if let Some(link) = page_link("channel", id) {
        items.push(Item::new(icons::LINK_2, t("Share"), Command::Copy(link)));
    }

    items.sort_by_key(|item| item.danger);
    items
}

pub fn playlist(app: &Jondo, playlist: &Playlist, cx: &App) -> Vec<Item> {
    let tracks: Vec<Song> = app
        .songs(&playlist.id, cx)
        .iter()
        .filter(|song| !song.broken)
        .cloned()
        .collect();
    let mut items = Vec::new();

    if !playlist.is_local() && !playlist.liked() && !playlist.radio() {
        let saved = app.is_saved(&playlist.id);
        items.push(Item::new(
            icons::HEART,
            if saved {
                t("Unfollow playlist")
            } else {
                t("Follow playlist")
            },
            Command::PlaylistSave(playlist.clone()),
        ));
    }

    if let Some(link) = page_link("playlist", &playlist.id) {
        items.push(Item::new(icons::LINK_2, t("Share"), Command::Copy(link)));
    }

    if !tracks.is_empty() {
        let all = tracks
            .iter()
            .all(|song| app.selection.iter().any(|id| id == &song.id));
        items.push(Item::new(
            icons::LIST_PLUS,
            t("Add to queue"),
            Command::Enqueue(tracks.clone()),
        ));
        items.push(Item::new(
            icons::SQUARE_PLUS,
            t("Add to playlist"),
            Command::Pick(tracks.clone()),
        ));
        items.push(Item::new(
            if all {
                icons::SQUARE
            } else {
                icons::LIST_CHECKS
            },
            if all {
                t("Deselect all")
            } else {
                t("Select all")
            },
            if all {
                Command::Clear
            } else {
                Command::Select(tracks)
            },
        ));
    }

    if playlist.is_local() {
        items.push(Item::new(
            icons::PENCIL,
            t("Rename playlist"),
            Command::PlaylistRename(playlist.id.clone(), playlist.name.clone()),
        ));
        items.push(Item::danger(
            icons::TRASH_2,
            t("Delete playlist"),
            Command::PlaylistDelete(playlist.id.clone()),
        ));
    }

    items.sort_by_key(|item| item.danger);
    items
}

pub fn album_card(app: &Jondo, album: &crate::shared::music::domain::Album, cx: &App) -> Vec<Item> {
    let playing = app.album(&album.id, cx).is_some_and(|loaded| {
        app.player.read(cx).playing
            && app
                .player
                .read(cx)
                .current()
                .is_some_and(|song| loaded.tracks.iter().any(|item| item.id == song.id))
    });
    let mut items = vec![Item::new(
        if playing {
            icons::PAUSE_LINE
        } else {
            icons::PLAY_LINE
        },
        if playing { t("Pause") } else { t("Play") },
        if playing {
            Command::Toggle
        } else {
            Command::AlbumPlay(album.id.clone())
        },
    )];
    if let Some(link) = page_link("album", &album.id) {
        items.push(Item::new(icons::LINK_2, t("Share"), Command::Copy(link)));
    }

    items.sort_by_key(|item| item.danger);
    items
}

pub fn shelf(app: &Jondo, playlist: &Playlist, cx: &App) -> Vec<Item> {
    let mut items = Vec::new();
    let playing = app.player.read(cx).playing
        && app.player.read(cx).current().is_some_and(|song| {
            app.songs(&playlist.id, cx)
                .iter()
                .any(|item| item.id == song.id)
        });
    items.push(Item::new(
        if playing {
            icons::PAUSE_LINE
        } else {
            icons::PLAY_LINE
        },
        if playing { t("Pause") } else { t("Play") },
        Command::PlaylistPlay(playlist.id.clone()),
    ));

    if !playlist.is_local() && !playlist.liked() && !playlist.radio() {
        let saved = app.is_saved(&playlist.id);
        items.push(Item::new(
            icons::HEART,
            if saved {
                t("Unfollow playlist")
            } else {
                t("Follow playlist")
            },
            Command::PlaylistSave(playlist.clone()),
        ));
    }

    if let Some(link) = page_link("playlist", &playlist.id) {
        items.push(Item::new(icons::LINK_2, t("Share"), Command::Copy(link)));
    }

    if playlist.is_local() {
        items.push(Item::danger(
            icons::TRASH_2,
            t("Remove"),
            Command::PlaylistDelete(playlist.id.clone()),
        ));
    }

    items.sort_by_key(|item| item.danger);
    items
}

pub fn queue(app: &Jondo, cx: &App, index: usize) -> Vec<Item> {
    let songs = &app.player.read(cx).queue.songs;
    let Some(song) = songs.get(index) else {
        return Vec::new();
    };

    let mut items = Vec::new();
    if index + 1 < songs.len() {
        items.push(Item::new(
            icons::CHEVRON_DOWN,
            t("Move down"),
            Command::QueueDown(index),
        ));
    }

    if index != app.player.read(cx).queue.at && index != app.player.read(cx).queue.at + 1 {
        items.push(Item::new(
            icons::ARROW_UP_FROM_LINE,
            t("Below current"),
            Command::QueueSnap(index),
        ));
    }

    if let Some(link) = spotify_link(song) {
        items.push(Item::new(
            icons::Icon::Spotify,
            t("Share Spotify"),
            Command::Copy(link),
        ));
    }

    if let Some(link) = youtube_link(song) {
        items.push(Item::new(
            icons::Icon::YouTube,
            t("Share YouTube"),
            Command::Copy(link),
        ));
    }

    items.push(Item::danger(
        icons::TRASH_2,
        t("Remove"),
        Command::QueueRemove(index),
    ));
    items.sort_by_key(|item| item.danger);
    items
}

pub fn trigger<F>(
    cx: &mut Context<Jondo>,
    id: impl Into<ElementId>,
    build: F,
    bottom: f32,
) -> Stateful<Div>
where
    F: Fn(&mut Jondo, &App) -> (Option<SharedString>, Vec<Item>) + 'static,
{
    trigger_in(Scope::Both, cx, id, 28.0, build, bottom)
}

pub fn trigger_in<F>(
    scope: Scope,
    cx: &mut Context<Jondo>,
    id: impl Into<ElementId>,
    size: f32,
    build: F,
    bottom: f32,
) -> Stateful<Div>
where
    F: Fn(&mut Jondo, &App) -> (Option<SharedString>, Vec<Item>) + 'static,
{
    let id: ElementId = id.into();
    let fade = hover::value(&id, std::time::Instant::now());
    div()
        .id(id.clone())
        .debug_selector({
            let id = id.clone();
            move || id.to_string()
        })
        .w(theme::unit(size))
        .h(theme::unit(size))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(theme::unit(6.0))
        .cursor_pointer()
        .when(fade > 0.0, |this| {
            this.bg(theme::alpha(theme::ZINC_800, 0.8 * fade))
        })
        .on_hover({
            let id = id.clone();
            move |on: &bool, window, cx| {
                hover::set_in(
                    scope,
                    &id,
                    *on,
                    std::time::Instant::now(),
                    window.last_input_was_touch(),
                );
                paint::paint(scope, cx);
            }
        })
        .on_click(cx.listener(move |app, event: &gpui::ClickEvent, _, cx| {
            cx.stop_propagation();
            let (header, items) = build(app, cx);
            if items.is_empty() {
                return;
            }

            app.menu = Some(Menu::context(event.position(), header, items, bottom));
            app.notify_shell(cx);
        }))
        .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .child(icons::icon(
            icons::ELLIPSIS_VERTICAL,
            16.0,
            theme::mix(theme::MUTED, theme::rgb(0xffffff), fade),
        ))
        .tooltip(move |window, cx| Tooltip::new(t("More actions")).build(window, cx))
}

pub fn page_link(page: &str, id: &str) -> Option<String> {
    if page == "channel" {
        let id = id.strip_prefix("youtube:").unwrap_or(id).trim();
        if id.is_empty() {
            return None;
        }

        return Some(if id.starts_with('@') {
            format!("https://www.youtube.com/{id}")
        } else {
            format!("https://www.youtube.com/channel/{id}")
        });
    }

    if page == "playlist"
        && let Some(list) = id.strip_prefix("youtube:playlist:")
    {
        return Some(format!("https://www.youtube.com/playlist?list={list}"));
    }

    let spotify = id.strip_prefix("spotify:").unwrap_or(id);
    if spotify.len() < 21
        || spotify.len() > 22
        || !spotify.chars().all(|c| c.is_ascii_alphanumeric())
    {
        return None;
    }

    Some(format!("https://open.spotify.com/{page}/{spotify}"))
}

fn radio_id(id: &str) -> String {
    let track = id.strip_prefix("spotify:track:").unwrap_or(id);
    format!("spotify:radio:track:{track}")
}

fn spotify_link(song: &Song) -> Option<String> {
    if song.source != crate::shared::music::domain::Source::Spotify {
        return None;
    }

    let id = song.id.strip_prefix("spotify:track:").unwrap_or(&song.id);
    if id.len() < 21 || id.len() > 22 {
        return None;
    }

    Some(format!("https://open.spotify.com/track/{id}"))
}

fn youtube_link(song: &Song) -> Option<String> {
    if song.source == crate::shared::music::domain::Source::Local {
        return None;
    }

    song.video()
        .map(|id| format!("https://www.youtube.com/watch?v={id}"))
}

#[cfg(test)]
#[path = "actions.test.rs"]
mod tests;
