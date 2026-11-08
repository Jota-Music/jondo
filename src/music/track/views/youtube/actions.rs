use std::time::Instant;

use gpui::{App, Context, Window};
use gpui_kit::component::input::InputEvent;
use url::Url;

use crate::shared::i18n::t;
use crate::shared::music::domain::Song;
use crate::shell::app::{Event, Jondo};

use super::{Editor, Mode, submit};

pub fn wire(app: &mut Jondo, window: &mut Window, cx: &mut Context<Jondo>) {
    let link = app.field("youtube-link", "", t("ID or URL"), window, cx);
    app.subscriptions.push(cx.subscribe_in(
        &link,
        window,
        |app, state, event: &InputEvent, window, cx| match event {
            InputEvent::Change => {
                app.ui.editor.link = state.read(cx).value().to_string();
                app.ui.editor.load = false;
                let raw = app.ui.editor.link.trim().to_owned();
                let resolved = parse_video(stripped(&raw));
                app.ui.editor.resolving = resolved.as_deref().is_some_and(|id| id != raw.as_str());
                app.ui.editor.link_at = app.ui.editor.resolving.then(Instant::now);
                cx.notify();
            }
            InputEvent::PressEnter { .. } => {
                app.ui.editor.link = state.read(cx).value().to_string();
                let _ = window;
                save(app, cx);
                cx.notify();
            }
            _ => {}
        },
    ));

    let search = app.field(
        "youtube-search",
        "",
        t("Song, artist or link..."),
        window,
        cx,
    );
    app.subscriptions.push(cx.subscribe_in(
        &search,
        window,
        |app, state, event: &InputEvent, _, cx| match event {
            InputEvent::Change => {
                app.ui.editor.search = state.read(cx).value().to_string();
                app.ui.editor.search_at = Some(Instant::now());
                cx.notify();
            }
            InputEvent::PressEnter { .. } => {
                let value = state.read(cx).value().trim().to_owned();
                app.ui.editor.search_at = None;
                submit(app, value, cx);
                cx.notify();
            }
            _ => {}
        },
    ));
}

pub fn switch(app: &mut Jondo, mode: Mode, window: &mut Window, cx: &mut Context<Jondo>) {
    if app.ui.editor.mode == mode {
        app.ui.editor.focus = true;
        return;
    }

    if mode == Mode::Assign {
        let preview = &mut app.preview;
        app.player.update(cx, |player, _| preview.stop(player));
        app.ui.editor.search_at = None;
    }

    app.ui.editor.mode = mode;
    app.ui.editor.focus = true;
    app.ui
        .editor
        .slide
        .start(if mode == Mode::Search { 1.0 } else { 0.0 }, Instant::now());

    if mode != Mode::Search {
        return;
    }

    let value = {
        let editor = &mut app.ui.editor;
        if editor.search.trim().is_empty()
            && let Some(song) = &editor.song
        {
            let mut terms = song.name.clone();
            for artist in &song.artists {
                if !artist.is_empty() {
                    terms.push(' ');
                    terms.push_str(artist);
                }
            }

            editor.search = terms;
        }

        editor.search.trim().to_owned()
    };

    app.set_field("youtube-search", &value, window, cx);
    submit(app, value, cx);
}

pub(super) fn search(app: &mut Jondo, query: String) {
    let youtube = app.youtube.clone();
    app.jobs.spawn(async move {
        let result = youtube.search(&query).await;
        Event::YouTubeVideos(query, result)
    });
}

pub fn save(app: &mut Jondo, cx: &mut Context<Jondo>) {
    let Some(song) = app.ui.editor.song.clone() else {
        return;
    };

    let raw = stripped(&app.ui.editor.link).to_owned();
    let id = match parse_video(&raw) {
        Some(id) => id,
        None if raw.is_empty() => String::new(),
        None => {
            app.fail(t("Paste a valid YouTube video ID or URL."));
            return;
        }
    };

    let before = app
        .ui
        .editor
        .song
        .as_ref()
        .and_then(video_of)
        .unwrap_or_default();
    if id == before {
        app.close_editor(cx);
        return;
    }

    let current = app
        .player
        .read(cx)
        .current()
        .is_some_and(|value| value.id == song.id);
    let position = app.player.read(cx).progress;
    let playing = app.player.read(cx).playing || app.preview.holding();

    apply_video(app, &song.id, &id, cx);
    app.library_changed(cx);

    if current {
        app.preview.interrupt();
        app.load_current_at(if playing { 0.0 } else { position }, playing, cx);
    } else {
        let preview = &mut app.preview;
        app.player.update(cx, |player, _| preview.stop(player));
    }

    app.close_editor(cx);
}

fn apply_video(app: &mut Jondo, song_id: &str, video: &str, cx: &mut Context<Jondo>) {
    app.store.pick(song_id, video);
    app.player.update(cx, |player, _| {
        for song in &mut player.queue.songs {
            set_video(song, song_id, video);
        }
    });

    for playlist in &mut app.store.playlists {
        for song in &mut playlist.songs {
            set_video(song, song_id, video);
        }
    }

    for playlist in &mut app.store.saved {
        for song in &mut playlist.songs {
            set_video(song, song_id, video);
        }
    }

    app.catalog.update(cx, |catalog, _| {
        for playlist in catalog.playlists.values_mut() {
            for song in &mut playlist.songs {
                set_video(song, song_id, video);
            }
        }
    });

    if let Some(song) = &mut app.store.last_song {
        set_video(song, song_id, video);
    }
}

fn set_video(song: &mut Song, song_id: &str, video: &str) {
    if song.id != song_id {
        return;
    }

    song.video_id = (!video.is_empty()).then(|| video.to_owned());
    if video.is_empty() {
        song.uri = None;
    }
}

pub fn changed(editor: &Editor) -> bool {
    let current = editor
        .song
        .as_ref()
        .and_then(|song| song.video())
        .unwrap_or_default();
    editor.link != current
}

pub fn video_of(song: &Song) -> Option<String> {
    song.video()
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

pub(super) fn stored(app: &Jondo, id: &str, cx: &App) -> Option<String> {
    fn find(songs: &[Song], id: &str) -> Option<String> {
        songs.iter().find(|song| song.id == id).and_then(video_of)
    }

    find(&app.player.read(cx).queue.songs, id)
        .or_else(|| {
            app.store
                .playlists
                .iter()
                .find_map(|playlist| find(&playlist.songs, id))
        })
        .or_else(|| {
            app.store
                .saved
                .iter()
                .find_map(|playlist| find(&playlist.songs, id))
        })
        .or_else(|| {
            app.catalog
                .read(cx)
                .playlists
                .values()
                .find_map(|playlist| find(&playlist.songs, id))
        })
        .or_else(|| app.store.youtube_links.get(id).cloned())
}

pub fn stripped(text: &str) -> &str {
    let text = text.trim();
    match text.get(..8) {
        Some(prefix) if prefix.eq_ignore_ascii_case("youtube:") => &text[8..],
        _ => text,
    }
}

pub fn parse_video(query: &str) -> Option<String> {
    let text = stripped(query);
    if text.is_empty() {
        return None;
    }

    if is_video(text) {
        return Some(text.to_owned());
    }

    let url = link(text)?;
    let host = host(&url)?;
    if !matches!(
        host.as_str(),
        "youtube.com" | "youtu.be" | "youtube-nocookie.com"
    ) {
        return None;
    }

    if let Some(value) = url
        .query_pairs()
        .find(|(key, _)| key == "v")
        .map(|(_, value)| value.into_owned())
        && is_video(&value)
    {
        return Some(value);
    }

    if host == "youtu.be" {
        let id = url.path().trim_start_matches('/');
        if is_video(id) {
            return Some(id.to_owned());
        }
    }

    let mut parts = url.path().trim_start_matches('/').split('/');
    let kind = parts.next().unwrap_or_default();
    let id = parts.next().unwrap_or_default();
    if matches!(kind, "shorts" | "embed" | "live") && is_video(id) {
        return Some(id.to_owned());
    }

    None
}

fn link(text: &str) -> Option<Url> {
    let text = if text.contains("://") {
        text.to_owned()
    } else {
        format!("https://{text}")
    };

    Url::parse(&text).ok()
}

fn host(url: &Url) -> Option<String> {
    let host = url.host_str()?;
    for prefix in ["www.", "music.", "m."] {
        if let Some(rest) = host.strip_prefix(prefix) {
            return Some(rest.to_owned());
        }
    }

    Some(host.to_owned())
}

fn is_video(value: &str) -> bool {
    value.len() == 11
        && value
            .chars()
            .all(|value| value.is_ascii_alphanumeric() || value == '-' || value == '_')
}
