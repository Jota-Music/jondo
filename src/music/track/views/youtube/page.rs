use gpui::{Context, Window};

use crate::shared::music::domain::Song;
use crate::shell::app::Jondo;

use super::actions::{parse_video, search, stored, stripped, video_of};
use super::{LINK_DELAY, SEARCH_DELAY};
use crate::music::track::app::Editor;
use crate::music::track::domain::Mode;

pub fn open(app: &mut Jondo, cx: &mut Context<Jondo>) {
    let Some(song) = app.player.read(cx).current().cloned() else {
        return;
    };

    edit(app, song, cx);
}

pub fn edit(app: &mut Jondo, song: Song, cx: &mut Context<Jondo>) {
    let video = video_of(&song)
        .or_else(|| stored(app, &song.id, cx))
        .unwrap_or_default();
    let preview = &mut app.preview;
    app.player.update(cx, |player, _| preview.stop(player));
    app.close_player();
    app.ui.closing = None;
    app.identify(&song);
    app.ui.editor = Editor {
        open: true,
        song: Some(song),
        load: video.is_empty(),
        link: video,
        focus: true,
        ..Default::default()
    };
}

pub fn tick(app: &mut Jondo, window: &mut Window, cx: &mut Context<Jondo>) {
    if !app.ui.editor.open {
        return;
    }

    if let Some(at) = app.ui.editor.link_at
        && at.elapsed() >= LINK_DELAY
    {
        let link = stripped(&app.ui.editor.link).to_owned();
        if let Some(id) = parse_video(&link) {
            app.ui.editor.link = id.clone();
            app.set_field("youtube-link", &id, window, cx);
        }

        app.ui.editor.resolving = false;
        app.ui.editor.link_at = None;
    }

    if app.ui.editor.load && app.ui.editor.link.is_empty() {
        let id = app.ui.editor.song.as_ref().map(|song| song.id.clone());
        if let Some(video) = id.and_then(|id| stored(app, &id, cx)) {
            app.ui.editor.link = video.clone();
            app.ui.editor.load = false;
            app.set_field("youtube-link", &video, window, cx);
        }
    }

    if app.ui.editor.mode == Mode::Search
        && let Some(at) = app.ui.editor.search_at
        && at.elapsed() >= SEARCH_DELAY
    {
        app.ui.editor.search_at = None;
        let value = app.ui.editor.search.trim().to_owned();
        submit(app, value, cx);
    }
}

pub(super) fn submit(app: &mut Jondo, value: String, cx: &mut Context<Jondo>) {
    if value == app.ui.editor.query {
        return;
    }

    let preview = &mut app.preview;
    app.player.update(cx, |player, _| preview.stop(player));
    app.ui.editor.query = value.clone();
    app.ui.editor.results.reset();
    app.ui.editor.results.loading = !value.is_empty();
    if !value.is_empty() {
        search(app, value);
    }
}
