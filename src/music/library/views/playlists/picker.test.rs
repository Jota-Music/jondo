use std::path::PathBuf;

use gpui::{TestAppContext, VisualTestContext};

use crate::shared::music::domain::Song;
use crate::shared::storage::Store;
use crate::shared::ui::views::design::theme;
use crate::shell::app::Jondo;

fn home() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("jondo-picker-{}", std::process::id()));
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

#[gpui::test]
fn enter_creates_the_playlist_and_adds_the_selection(cx: &mut TestAppContext) {
    home();
    cx.update(|cx| {
        gpui_kit::init(cx);
        theme::install(cx);
    });

    let (app, cx) = cx.add_window_view(|window, cx| Jondo::boot(Store::load(), window, cx));
    cx.run_until_parked();
    frame(cx);

    app.update(cx, |app, cx| {
        app.ui.picker = Some(vec![Song {
            id: "local:song:enter".to_owned(),
            name: "Enter".to_owned(),
            ..Default::default()
        }]);
        cx.notify();
    });
    cx.run_until_parked();
    frame(cx);

    let field = app.read_with(cx, |app, _| app.fields["picker-name"].clone());
    cx.update(|window, cx| {
        app.update(cx, |app, cx| {
            app.set_field("picker-name", "Typed", window, cx)
        });
        field.update(cx, |state, cx| state.focus(window, cx));
    });
    frame(cx);

    cx.simulate_keystrokes("enter");
    cx.run_until_parked();

    app.read_with(cx, |app, _| {
        let playlist = app
            .store
            .playlists
            .iter()
            .find(|playlist| playlist.name == "Typed")
            .expect("Enter must create the named playlist");
        assert!(
            playlist
                .songs
                .iter()
                .any(|song| song.id == "local:song:enter"),
            "the selected song must land in the new playlist"
        );
        assert!(
            app.ui.closing.is_some(),
            "the sheet must close after adding"
        );
    });
}
