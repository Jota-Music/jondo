use std::path::PathBuf;

use gpui::{Modifiers, MouseButton, TestAppContext, VisualTestContext};

use crate::shared::music::domain::Song;
use crate::shared::storage::{LocalPlaylist, Store};
use crate::shared::ui::views::design::theme;
use crate::shell::app::{Jondo, Route};

fn home() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("jondo-playlist-{}", std::process::id()));
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

fn click(cx: &mut VisualTestContext, selector: &'static str) {
    let center = cx
        .debug_bounds(selector)
        .unwrap_or_else(|| panic!("{selector} was not painted"))
        .center();
    cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
    cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
    cx.run_until_parked();
    frame(cx);
}

#[gpui::test]
fn pausing_from_the_header_keeps_the_playlist_where_it_is(cx: &mut TestAppContext) {
    home();
    let mut store = Store::load();
    let song = Song {
        id: "local:song:one".to_owned(),
        name: "One".to_owned(),
        duration: 180,
        ..Default::default()
    };
    store.playlists.push(LocalPlaylist {
        id: "local:pause".to_owned(),
        name: "Mix".to_owned(),
        songs: vec![song.clone()],
        modified: 0,
    });

    cx.update(|cx| {
        gpui_kit::init(cx);
        theme::install(cx);
    });

    let (app, cx) = cx.add_window_view(|window, cx| Jondo::boot(store, window, cx));
    cx.run_until_parked();

    app.update(cx, |app, cx| {
        app.player.update(cx, |player, _| {
            player.queue.set(vec![song], 0);
            player.loaded = true;
            player.playing = true;
            player.duration = 180.0;
            player.progress = 42.0;
        });
        app.open(Route::Playlist("local:pause".to_owned()), cx);
        cx.notify();
    });
    cx.run_until_parked();
    frame(cx);

    click(cx, "page-play");

    let (playing, progress) = app.read_with(cx, |app, cx| {
        let player = app.player.read(cx);
        (player.playing, player.progress)
    });
    assert!(!playing, "the header should pause instead of restarting");
    assert_eq!(progress, 42.0, "pausing must not rewind the playlist");
}
