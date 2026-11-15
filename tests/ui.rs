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
fn the_queue_numbers_stay_on_one_line(cx: &mut TestAppContext) {
    home();
    let mut store = jondo::shared::storage::Store::load();
    store.queue = (0..1200)
        .map(|index| jondo::shared::music::domain::Song {
            id: format!("spotify:track:{index}"),
            ..Default::default()
        })
        .collect();
    store.queue_index = 999;

    cx.update(|cx| {
        gpui_kit::init(cx);
        jondo::shared::ui::views::design::theme::install(cx);
    });

    let (app, cx) = cx.add_window_view(|window, cx| Jondo::boot(store, window, cx));
    cx.run_until_parked();
    frame(cx);

    app.update(cx, |app, cx| app.open_queue(cx));
    cx.run_until_parked();
    frame(cx);

    let three = bounds(cx, "queue-number-998").size.height;
    let four = bounds(cx, "queue-number-1000").size.height;
    assert_eq!(
        three, four,
        "a wider number must not wrap onto a second line"
    );
}

/// The layout the app promises: the search lives in the header on the right,
/// the player at the bottom. A dependency that changes how text measures or
/// how a box sizes moves any of it, and that shows up here.
#[gpui::test]
fn the_shell_keeps_its_layout(cx: &mut TestAppContext) {
    home();
    let store = jondo::shared::storage::Store::load();

    cx.update(|cx| {
        gpui_kit::init(cx);
        jondo::shared::ui::views::design::theme::install(cx);
    });

    let (_app, cx) = cx.add_window_view(|window, cx| Jondo::boot(store, window, cx));
    cx.run_until_parked();
    frame(cx);

    let size = cx.update(|window, _| window.viewport_size());
    let search = bounds(cx, "search-toggle");
    let play = bounds(cx, "bar-play");
    let filters = bounds(cx, "filter-all");
    eprintln!(
        "viewport {:?} filters {:?}",
        cx.update(|window, _| window.viewport_size()),
        filters
    );

    assert!(
        search.center().y < size.height / 4.0,
        "the search belongs in the header, not at {}",
        search.center().y
    );
    assert!(
        filters.center().y > search.center().y,
        "the shelf filters belong under the header"
    );
    assert!(
        play.center().y > filters.center().y,
        "the player belongs under the shelf"
    );
    assert!(
        play.center().y > size.height * 3.0 / 4.0,
        "the player belongs at the bottom, not at {}",
        play.center().y
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

/// The glow covers the band between the header and the player bar, which is
/// where the web shell paints it: the desktop player is part of that band, so
/// leaving it out blackens the top of the player for no reason.
#[gpui::test]
fn the_glow_reaches_the_player(cx: &mut TestAppContext) {
    home();
    let mut store = jondo::shared::storage::Store::load();
    store.queue = vec![jondo::shared::music::domain::Song {
        id: "spotify:track:one".to_owned(),
        ..Default::default()
    }];
    store.queue_index = 0;

    cx.update(|cx| {
        gpui_kit::init(cx);
        jondo::shared::ui::views::design::theme::install(cx);
    });

    let (_app, cx) = cx.add_window_view(|window, cx| Jondo::boot(store, window, cx));
    cx.run_until_parked();
    frame(cx);

    let glow = bounds(cx, "ambient-backdrop");
    let player = bounds(cx, "player-panel");
    assert!(
        glow.origin.y <= player.origin.y
            && glow.origin.y + glow.size.height >= player.origin.y + player.size.height,
        "the player must sit inside the glow, not under a black band"
    );
}

/// The ambient background is a switch over the same store the rest of the app
/// writes: flipping it must reach the file, or the glow comes back on launch.
#[gpui::test]
fn the_ambient_background_switch_persists(cx: &mut TestAppContext) {
    home();
    let store = jondo::shared::storage::Store::load();
    assert!(store.backdrop, "the glow is on by default");

    cx.update(|cx| {
        gpui_kit::init(cx);
        jondo::shared::ui::views::design::theme::install(cx);
    });

    let (app, cx) = cx.add_window_view(|window, cx| Jondo::boot(store, window, cx));
    cx.run_until_parked();

    let handle = cx.update(|window, _| window.window_handle());
    cx.simulate_window_resize(handle, gpui::size(gpui::px(390.0), gpui::px(844.0)));
    frame(cx);

    assert!(
        cx.debug_bounds("ambient-backdrop").is_some(),
        "the glow must paint behind the page"
    );
    assert!(
        cx.debug_bounds("ambient-dust").is_some(),
        "the field of motes rides over the glow"
    );

    // Settings scans the browsers on the blocking pool when it opens, so the
    // frame needs the parking mode the other settings test uses.
    cx.executor().allow_parking();
    app.update(cx, |app, cx| app.open(Route::Settings, cx));
    cx.run_until_parked();
    frame(cx);

    assert!(
        cx.debug_bounds("settings-item-0").is_some(),
        "the settings page should have painted"
    );

    // The list only builds the sections under the viewport, and the account and
    // language cards fill a phone's first screen.
    let at = gpui::point(gpui::px(195.0), gpui::px(400.0));
    for _ in 0..6 {
        cx.simulate_event(gpui::ScrollEvent {
            position: at,
            delta: gpui::ScrollDelta::Pixels(gpui::point(gpui::px(0.0), gpui::px(-400.0))),
            ..Default::default()
        });
        frame(cx);
    }

    assert!(
        cx.debug_bounds("settings-visuals").is_some(),
        "the visuals section belongs on the settings page"
    );

    // The knobs step from the number on screen, and each step lands in the
    // store: the buttons are not decoration around the field.
    cx.simulate_event(gpui::ScrollEvent {
        position: at,
        delta: gpui::ScrollDelta::Pixels(gpui::point(gpui::px(0.0), gpui::px(300.0))),
        ..Default::default()
    });
    frame(cx);

    let before = jondo::shared::storage::Store::load().glow;
    click(cx, "visuals-glow-increase");
    assert_eq!(
        jondo::shared::storage::Store::load().glow,
        before + 1,
        "the up button must raise the glow"
    );
    click(cx, "visuals-glow-decrease");
    assert_eq!(
        jondo::shared::storage::Store::load().glow,
        before,
        "the down button must lower it back"
    );

    // The field only takes digits: a letter is refused before it reaches the
    // store, and the number on screen is left untouched.
    click(cx, "visuals-glow-input");
    let field = cx.update(|window, cx| {
        app.update(cx, |app, cx| app.field("visuals-glow", "", "", window, cx))
    });
    assert_eq!(
        field.read_with(cx, |state, _| state.selected_text().to_string()),
        before.to_string(),
        "a press must select the number so a keystroke replaces it"
    );
    cx.simulate_keystrokes("a");
    cx.run_until_parked();
    assert_eq!(
        app.read_with(cx, |app, cx| app.value("visuals-glow", cx)),
        before.to_string(),
        "a letter must not reach the field"
    );

    app.update(cx, |app, cx| {
        app.set_backdrop(false, cx);
        cx.notify();
    });
    frame(cx);

    assert!(
        cx.debug_bounds("ambient-backdrop").is_none(),
        "turning the switch off must drop the glow"
    );
    assert!(
        cx.debug_bounds("ambient-dust").is_none(),
        "the motes go with it"
    );
    assert!(
        !jondo::shared::storage::Store::load().backdrop,
        "the switch must survive a reload"
    );

    // With the glow off the knobs are inert: a press must not write a value
    // behind a setting that is not painting anything.
    let off = jondo::shared::storage::Store::load().glow;
    click(cx, "visuals-glow-increase");
    assert_eq!(
        jondo::shared::storage::Store::load().glow,
        off,
        "a disabled stepper must not change the store"
    );

    // The two knobs are percentages over the same store: each one holds the
    // range its field reads well in, so no keystroke can paint a glow away.
    app.update(cx, |app, cx| {
        app.set_glow(90, cx);
        app.set_mote(1, cx);
        cx.notify();
    });
    frame(cx);

    let store = jondo::shared::storage::Store::load();
    assert_eq!(
        (store.glow, store.mote),
        (50, 40),
        "a knob clamps to its own range"
    );
}

/// The phone frame is Jota's: the page heads carry 12, the shelf hangs its
/// cards on 8, and settings uses its own 16. The shell must not add an inset of
/// its own on top of any of them.
#[gpui::test]
fn the_phone_page_keeps_jotas_insets(cx: &mut TestAppContext) {
    home();
    let store = jondo::shared::storage::Store::load();

    cx.update(|cx| {
        gpui_kit::init(cx);
        jondo::shared::ui::views::design::theme::install(cx);
    });

    let (app, cx) = cx.add_window_view(|window, cx| Jondo::boot(store, window, cx));
    cx.run_until_parked();

    let handle = cx.update(|window, _| window.window_handle());
    cx.simulate_window_resize(handle, gpui::size(gpui::px(390.0), gpui::px(844.0)));

    let id = app.update(cx, |app, cx| {
        app.create_playlist("Mix", cx);
        app.open(Route::Home, cx);
        cx.notify();
        app.local_playlists()
            .first()
            .expect("the playlist must be in the store")
            .id
            .clone()
    });
    cx.run_until_parked();
    frame(cx);

    let id: &'static str = Box::leak(id.into_boxed_str());
    let filters = bounds(cx, "filter-all");
    // The segment sits one border inside the bar's `px-3`.
    assert_at(filters.origin.x, 13.0, "the shelf bar's head inset");

    let card = cx
        .debug_bounds(&*Box::leak(format!("playlist-card-{id}").into_boxed_str()))
        .expect("the shelf must paint the playlist it holds");
    assert_at(card.origin.x, 8.0, "the shelf's pl-2");

    cx.executor().allow_parking();
    app.update(cx, |app, cx| app.open(Route::Settings, cx));
    cx.run_until_parked();
    frame(cx);

    let section = bounds(cx, "settings-item-0");
    assert_at(section.origin.x, 16.0, "settings' px-4");
}

/// A track list is full bleed on the phone: the row starts at the page edge
/// and the panel's gutter alone shortens it on the right. Any shell inset on
/// top of that is the bug this guards.
#[gpui::test]
fn a_phone_track_list_runs_edge_to_edge(cx: &mut TestAppContext) {
    home();
    let mut store = jondo::shared::storage::Store::load();
    store.playlists.push(jondo::shared::storage::LocalPlaylist {
        id: "local:one".to_owned(),
        name: "Mix".to_owned(),
        songs: vec![jondo::shared::music::domain::Song {
            id: "spotify:track:one".to_owned(),
            name: "One".to_owned(),
            ..Default::default()
        }],
        modified: 0,
    });
    store.save();

    cx.update(|cx| {
        gpui_kit::init(cx);
        jondo::shared::ui::views::design::theme::install(cx);
    });

    let (app, cx) = cx.add_window_view(|window, cx| Jondo::boot(store, window, cx));
    cx.run_until_parked();

    let handle = cx.update(|window, _| window.window_handle());
    cx.simulate_window_resize(handle, gpui::size(gpui::px(390.0), gpui::px(844.0)));
    app.update(cx, |app, cx| {
        app.open(Route::Playlist("local:one".to_owned()), cx)
    });
    cx.run_until_parked();
    frame(cx);

    let row = bounds(cx, "song-row-0");
    assert_at(row.origin.x, 0.0, "the track row's page edge");
    let right = row.origin.x + row.size.width;
    let gutter = gpui::px(390.0) - jondo::shared::ui::views::design::theme::unit(16.0);
    assert!(
        (right.as_f32() - gutter.as_f32()).abs() < 0.5,
        "the track row ends on the panel's pr-4 gutter: expected {gutter:?}, got {right:?}"
    );
}

/// Layout snaps to device pixels, so a design offset lands within half of one
/// of `theme::unit`'s value.
fn assert_at(actual: gpui::Pixels, design: f32, what: &str) {
    let expected = jondo::shared::ui::views::design::theme::unit(design);
    assert!(
        (actual.as_f32() - expected.as_f32()).abs() < 0.5,
        "{what}: expected {expected:?}, got {actual:?}"
    );
}
