use std::path::PathBuf;

use gpui::{TestAppContext, VisualTestContext};

use crate::shared::music::domain::Song;
use crate::shared::storage::{LocalPlaylist, Store};
use crate::shared::ui::views::design::theme;
use crate::shell::app::{Jondo, Route};

fn home() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("jondo-actions-{}", std::process::id()));
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

fn page(app: &mut Jondo, cx: &mut gpui::Context<Jondo>) {
    app.open(Route::Playlist("local:back".to_owned()), cx);
    cx.notify();
}

#[gpui::test]
fn escape_steps_to_the_previous_page(cx: &mut TestAppContext) {
    home();
    cx.update(|cx| {
        gpui_kit::init(cx);
        theme::install(cx);
    });

    let (app, cx) = cx.add_window_view(|window, cx| Jondo::boot(Store::load(), window, cx));
    cx.run_until_parked();
    app.update(cx, page);
    cx.run_until_parked();
    frame(cx);

    cx.simulate_keystrokes("escape");
    cx.run_until_parked();

    assert_eq!(app.read_with(cx, |app, _| app.route.clone()), Route::Home);
}

#[gpui::test]
fn escape_closes_an_overlay_before_leaving_the_page(cx: &mut TestAppContext) {
    home();
    cx.update(|cx| {
        gpui_kit::init(cx);
        theme::install(cx);
    });

    let (app, cx) = cx.add_window_view(|window, cx| Jondo::boot(Store::load(), window, cx));
    cx.run_until_parked();
    app.update(cx, page);
    app.update(cx, |app, _| app.open_search());
    cx.run_until_parked();
    frame(cx);

    cx.simulate_keystrokes("escape");
    cx.run_until_parked();

    let (route, search) = app.read_with(cx, |app, _| (app.route.clone(), app.ui.search_open));
    assert_eq!(route, Route::Playlist("local:back".to_owned()));
    assert!(!search, "back should collapse the search panel first");
}

#[gpui::test]
fn repeat_mode_survives_a_restart(cx: &mut TestAppContext) {
    home();
    let mut store = Store::load();
    store.repeat = "all".to_owned();

    cx.update(|cx| {
        gpui_kit::init(cx);
        theme::install(cx);
    });

    let (app, cx) = cx.add_window_view(|window, cx| Jondo::boot(store, window, cx));
    cx.run_until_parked();

    assert_eq!(
        app.read_with(cx, |app, cx| app.player.read(cx).repeat),
        crate::music::player::Repeat::All,
        "the saved repeat mode is restored"
    );

    app.update(cx, |app, cx| app.cycle_repeat(cx));
    assert_eq!(
        app.read_with(cx, |app, _| app.store.repeat.clone()),
        "one",
        "cycling the mode saves it"
    );
}

#[gpui::test]
fn space_types_into_a_focused_field(cx: &mut TestAppContext) {
    home();
    let mut store = Store::load();
    store.playlists.push(LocalPlaylist {
        id: "local:typing".to_owned(),
        name: "Mix".to_owned(),
        songs: vec![Song {
            id: "local:song:one".to_owned(),
            name: "One".to_owned(),
            ..Default::default()
        }],
        modified: 0,
    });

    cx.update(|cx| {
        gpui_kit::init(cx);
        theme::install(cx);
    });

    let (app, cx) = cx.add_window_view(|window, cx| Jondo::boot(store, window, cx));
    cx.run_until_parked();
    app.update(cx, |app, cx| {
        app.open(Route::Playlist("local:typing".to_owned()), cx);
        cx.notify();
    });
    cx.run_until_parked();
    frame(cx);

    let field = app.read_with(cx, |app, _| app.fields["playlist-search"].clone());
    cx.update(|window, cx| field.update(cx, |state, cx| state.focus(window, cx)));
    frame(cx);

    cx.simulate_keystrokes("space");
    cx.run_until_parked();

    assert_eq!(
        field.read_with(cx, |state, _| state.value().to_string()),
        " ",
        "space must reach the field instead of toggling playback"
    );
}
