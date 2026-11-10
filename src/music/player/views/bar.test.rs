use std::path::PathBuf;

use gpui::{TestAppContext, VisualTestContext, px};

use crate::shared::music::domain::Song;
use crate::shared::storage::{LocalPlaylist, Store};
use crate::shared::ui::views::design::theme;
use crate::shell::app::{Jondo, Route};

fn home() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("jondo-bar-{}", std::process::id()));
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

fn width(cx: &mut VisualTestContext, selector: &'static str) -> gpui::Pixels {
    cx.debug_bounds(selector)
        .unwrap_or_else(|| panic!("{selector} was not painted"))
        .size
        .width
}

/// The bar at a phone width: both extras and the transport stay targets a
/// finger can find, and the song still keeps room to read.
#[gpui::test]
fn the_phone_bar_keeps_finger_sized_controls(cx: &mut TestAppContext) {
    home();
    let mut store = Store::load();
    let song = Song {
        id: "local:song:touch".to_owned(),
        name: "A reasonably long song title".to_owned(),
        duration: 180,
        ..Default::default()
    };
    store.playlists.push(LocalPlaylist {
        id: "local:touch".to_owned(),
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
        });
        app.open(Route::Playlist("local:touch".to_owned()), cx);
        cx.notify();
    });
    cx.run_until_parked();

    cx.simulate_resize(gpui::size(px(412.0), px(800.0)));
    cx.run_until_parked();
    frame(cx);

    let touch = theme::unit(44.0);
    for selector in ["bar-more", "bar-expand", "bar-play"] {
        assert!(
            width(cx, selector) >= touch,
            "{selector} must stay a finger-sized target on a phone"
        );
    }

    assert!(
        width(cx, "bar-title") > theme::unit(40.0),
        "the two extras must not leave the song title unreadable"
    );

    let expand = cx.debug_bounds("bar-expand").unwrap().center();
    for expected in [true, false] {
        cx.simulate_mouse_down(expand, gpui::MouseButton::Left, gpui::Modifiers::default());
        cx.simulate_mouse_up(expand, gpui::MouseButton::Left, gpui::Modifiers::default());
        cx.run_until_parked();
        frame(cx);
        assert_eq!(
            app.read_with(cx, |app, _| app.ui.player_open),
            expected,
            "the bar chevron must toggle the player"
        );
    }
}
