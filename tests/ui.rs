//! The app mounted in a headless window: the tests that catch a refactor
//! changing what a page paints or what a click does.
//!
//! The store is pointed at a temporary home first, so a run never reads or
//! writes the real state. Nothing reaches the network: an empty store has no
//! session, and every fetch is skipped before it starts.

use std::path::PathBuf;

use gpui::{Modifiers, MouseButton, TestAppContext, VisualTestContext};
use gpui_pre as gpui;
use jondo::shell::app::{Jondo, Route};

/// A home under the temporary folder, pinned once for the whole binary.
fn home() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("jondo-ui-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    jondo::shared::storage::use_home(&dir);
    dir
}

fn frame(cx: &mut VisualTestContext) {
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
}

fn bounds(cx: &mut VisualTestContext, selector: &'static str) -> gpui::Bounds<gpui::Pixels> {
    cx.debug_bounds(selector)
        .unwrap_or_else(|| panic!("{selector} was not painted"))
}

fn click(cx: &mut VisualTestContext, selector: &'static str) {
    let center = bounds(cx, selector).center();
    cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
    cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
    cx.run_until_parked();
    frame(cx);
}

#[gpui::test]
fn the_shell_paints_on_a_fresh_home(cx: &mut TestAppContext) {
    home();
    let store = jondo::shared::storage::Store::load();

    cx.update(|cx| {
        gpui_kit::init(cx);
        jondo::shared::ui::views::design::theme::install(cx);
    });

    let (_app, cx) = cx.add_window_view(|window, cx| Jondo::boot(store, window, cx));
    cx.run_until_parked();
    frame(cx);

    assert!(bounds(cx, "search-toggle").size.width > gpui::px(0.0));
    assert!(bounds(cx, "filter-all").size.height > gpui::px(0.0));
}

#[gpui::test]
fn the_player_bar_shows_without_a_song(cx: &mut TestAppContext) {
    home();
    let store = jondo::shared::storage::Store::load();

    cx.update(|cx| {
        gpui_kit::init(cx);
        jondo::shared::ui::views::design::theme::install(cx);
    });

    let (_app, cx) = cx.add_window_view(|window, cx| Jondo::boot(store, window, cx));
    cx.run_until_parked();
    frame(cx);

    assert!(
        cx.debug_bounds("bar-play").is_some(),
        "the player bar should be painted even with an empty queue"
    );
}

#[gpui::test]
fn the_search_panel_opens_from_the_header(cx: &mut TestAppContext) {
    home();
    let store = jondo::shared::storage::Store::load();

    cx.update(|cx| {
        gpui_kit::init(cx);
        jondo::shared::ui::views::design::theme::install(cx);
    });

    let (_app, cx) = cx.add_window_view(|window, cx| Jondo::boot(store, window, cx));
    cx.run_until_parked();
    frame(cx);

    assert!(
        cx.debug_bounds("source-youtube").is_none(),
        "the panel starts closed"
    );

    click(cx, "search-toggle");
    assert!(
        cx.debug_bounds("source-youtube").is_some(),
        "the panel should be open"
    );
}

#[gpui::test]
fn the_create_sheet_opens_from_the_local_shelf(cx: &mut TestAppContext) {
    home();
    let store = jondo::shared::storage::Store::load();

    cx.update(|cx| {
        gpui_kit::init(cx);
        jondo::shared::ui::views::design::theme::install(cx);
    });

    let (_app, cx) = cx.add_window_view(|window, cx| Jondo::boot(store, window, cx));
    cx.run_until_parked();
    frame(cx);

    click(cx, "filter-local");
    click(cx, "new-playlist");
    assert!(
        cx.debug_bounds("create-submit").is_some(),
        "the create sheet should be open"
    );
}

#[gpui::test]
fn the_queue_opens_from_the_keyboard(cx: &mut TestAppContext) {
    home();
    let store = jondo::shared::storage::Store::load();

    cx.update(|cx| {
        gpui_kit::init(cx);
        jondo::shared::ui::views::design::theme::install(cx);
    });

    let (_app, cx) = cx.add_window_view(|window, cx| Jondo::boot(store, window, cx));
    cx.run_until_parked();
    frame(cx);

    cx.simulate_keystrokes("q");
    cx.run_until_parked();
    frame(cx);

    assert!(
        cx.debug_bounds("queue-sheet").is_some(),
        "q should open the queue"
    );
}

#[gpui::test]
fn opening_the_queue_queues_its_scroll(cx: &mut TestAppContext) {
    home();
    let store = jondo::shared::storage::Store::load();

    cx.update(|cx| {
        gpui_kit::init(cx);
        jondo::shared::ui::views::design::theme::install(cx);
    });

    let (app, cx) = cx.add_window_view(|window, cx| Jondo::boot(store, window, cx));
    cx.run_until_parked();
    frame(cx);

    app.update(cx, |app, cx| app.open_queue(cx));
    let scroll = app.update(cx, |app, _| app.scroll("queue"));

    assert!(
        scroll.0.borrow().deferred_scroll_to_item.is_some(),
        "opening the queue must queue its scroll without waiting for the clock"
    );
}

#[gpui::test]
fn a_long_local_shelf_virtualizes_its_rows(cx: &mut TestAppContext) {
    home();
    let store = jondo::shared::storage::Store::load();

    cx.update(|cx| {
        gpui_kit::init(cx);
        jondo::shared::ui::views::design::theme::install(cx);
    });

    let (app, cx) = cx.add_window_view(|window, cx| Jondo::boot(store, window, cx));
    cx.run_until_parked();
    frame(cx);

    app.update(cx, |app, cx| {
        for index in 0..30 {
            app.create_playlist(&format!("Mix {index}"), cx);
        }

        app.open(Route::Home, cx);
        cx.notify();
    });
    cx.run_until_parked();
    frame(cx);

    assert!(
        cx.debug_bounds("shelf-rows").is_some(),
        "the shelf must paint its virtualized rows"
    );

    let scroll = app.update(cx, |app, _| app.scroll("shelf"));
    assert!(
        scroll.0.borrow().base_handle.max_offset().y > gpui::px(0.0),
        "thirty playlists must overflow the shelf viewport"
    );
}
