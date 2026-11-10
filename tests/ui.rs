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

/// Settings is the longest route: one item per section, and only the ones under
/// the viewport are built. A page rebuilt whole on every scroll sample is what
/// made its scroll feel heavy on a phone.
#[gpui::test]
fn a_long_settings_page_virtualizes_its_sections(cx: &mut TestAppContext) {
    home();
    let store = jondo::shared::storage::Store::load();

    cx.update(|cx| {
        gpui_kit::init(cx);
        jondo::shared::ui::views::design::theme::install(cx);
    });

    let (app, cx) = cx.add_window_view(|window, cx| Jondo::boot(store, window, cx));
    cx.run_until_parked();

    // Opening Settings scans the browsers on the blocking pool, and a worker
    // thread waking the frame is exactly what parking mode is for.
    cx.executor().allow_parking();

    let handle = cx.update(|window, _| window.window_handle());
    cx.simulate_window_resize(handle, gpui::size(gpui::px(390.0), gpui::px(844.0)));
    frame(cx);

    app.update(cx, |app, cx| app.open(Route::Settings, cx));
    cx.run_until_parked();
    frame(cx);

    assert!(
        cx.debug_bounds("settings-item-0").is_some(),
        "the page opens on its header"
    );
    assert!(
        cx.debug_bounds("settings-item-9").is_none(),
        "the footer is below the phone's viewport, so it must not be built"
    );

    let at = gpui::point(gpui::px(195.0), gpui::px(400.0));
    for _ in 0..15 {
        cx.simulate_event(gpui::ScrollEvent {
            position: at,
            delta: gpui::ScrollDelta::Pixels(gpui::point(gpui::px(0.0), gpui::px(-400.0))),
            ..Default::default()
        });
        frame(cx);
    }

    assert!(
        cx.debug_bounds("settings-item-9").is_some(),
        "the end of the page must build its last section"
    );
    assert!(
        cx.debug_bounds("settings-item-0").is_none(),
        "the header is off screen, so the list must have dropped it"
    );
}
