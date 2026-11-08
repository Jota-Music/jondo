use std::path::PathBuf;

use gpui::{Modifiers, MouseButton, TestAppContext, VisualTestContext};

use crate::shared::music::domain::{Playlist, Source, UserView};
use crate::shared::storage::Store;
use crate::shared::ui::views::design::theme;
use crate::shell::app::{Jondo, Route};

fn home() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("jondo-user-{}", std::process::id()));
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
fn a_user_page_offers_the_generic_grid_and_list(cx: &mut TestAppContext) {
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
            catalog.users.ready(
                "alice".to_owned(),
                UserView {
                    username: "alice".to_owned(),
                    name: "Alice".to_owned(),
                    image: None,
                    playlists: vec![Playlist {
                        id: "spotify:playlist:abc".to_owned(),
                        name: "Mix".to_owned(),
                        source: Source::Spotify,
                        track_count: 1,
                        ..Default::default()
                    }],
                },
            );
        });
        let route = Route::User("alice".to_owned());
        app.history.open(route.clone());
        app.route = route;
        app.ui.user_grid = true;
        cx.notify();
    });
    cx.run_until_parked();
    frame(cx);
    frame(cx);

    assert!(
        cx.debug_bounds("user-view-grid").is_some(),
        "the generic grid option must be there"
    );
    assert!(
        cx.debug_bounds("user-view-list").is_some(),
        "the generic list option must be there"
    );
    assert!(
        cx.debug_bounds("playlist-card-spotify:playlist:abc")
            .is_some(),
        "the grid must paint the playlist card"
    );

    click(cx, "user-view-list");
    assert!(
        cx.debug_bounds("playlist-row-spotify:playlist:abc")
            .is_some(),
        "the list must paint the playlist row"
    );
    let grid = cx.update(|_, cx| app.read(cx).ui.user_grid);
    assert!(!grid, "list must become the choice");
}
