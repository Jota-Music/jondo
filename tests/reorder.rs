//! A reorder drag starts on a pressed row and retargets as the pointer moves
//! over the list. Returning to the item being dragged must make it the target
//! again, so releasing there drops nothing and ends the drag.

use std::path::PathBuf;

use gpui::{Modifiers, MouseButton, TestAppContext, VisualTestContext};
use gpui_pre as gpui;
use jondo::shared::music::domain::Song;
use jondo::shared::storage::{LocalPlaylist, Store};
use jondo::shell::app::Jondo;

fn home() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("jondo-reorder-{}", std::process::id()));
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

fn center(cx: &mut VisualTestContext, selector: String) -> gpui::Point<gpui::Pixels> {
    let selector: &'static str = Box::leak(selector.into_boxed_str());
    cx.debug_bounds(selector)
        .unwrap_or_else(|| panic!("{selector} was not painted"))
        .center()
}

fn names(app: &gpui::Entity<Jondo>, cx: &mut VisualTestContext) -> Vec<String> {
    app.update(cx, |app, cx| {
        app.shelf(cx)
            .iter()
            .map(|playlist| playlist.name.clone())
            .collect()
    })
}

#[gpui::test]
fn returning_to_the_dragged_card_keeps_the_order(cx: &mut TestAppContext) {
    home();
    let mut store = Store::load();
    for name in ["one", "two", "three"] {
        store.playlists.push(LocalPlaylist {
            id: format!("local:{name}"),
            name: name.to_owned(),
            songs: Vec::new(),
            modified: 0,
        });
    }

    cx.update(|cx| {
        gpui_kit::init(cx);
        jondo::shared::ui::views::design::theme::install(cx);
    });

    let (app, cx) = cx.add_window_view(|window, cx| Jondo::boot(store, window, cx));
    cx.run_until_parked();
    frame(cx);

    let one = center(cx, "playlist-card-local:one".to_owned());
    let two = center(cx, "playlist-card-local:two".to_owned());

    cx.simulate_mouse_down(one, MouseButton::Left, Modifiers::default());
    cx.simulate_mouse_move(two, MouseButton::Left, Modifiers::default());
    frame(cx);
    assert!(
        app.read_with(cx, |app, _| app.dragging()),
        "moving over another card must start the drag"
    );

    cx.simulate_mouse_move(one, MouseButton::Left, Modifiers::default());
    frame(cx);
    cx.simulate_mouse_up(one, MouseButton::Left, Modifiers::default());
    frame(cx);

    assert!(
        !app.read_with(cx, |app, _| app.dragging()),
        "releasing must end the drag"
    );
    assert!(
        cx.debug_bounds("shelf-rows").is_some(),
        "dropping a card must stay on the shelf instead of opening the card under the pointer"
    );
    assert_eq!(
        names(&app, cx),
        ["one", "two", "three"],
        "dropping a card back where it started must not reorder"
    );
}

#[gpui::test]
fn returning_to_the_dragged_queue_row_ends_the_drag(cx: &mut TestAppContext) {
    home();
    let mut store = Store::load();
    for name in ["one", "two", "three"] {
        store.queue.push(Song {
            id: format!("local:{name}"),
            name: name.to_owned(),
            duration: 180,
            ..Default::default()
        });
    }

    cx.update(|cx| {
        gpui_kit::init(cx);
        jondo::shared::ui::views::design::theme::install(cx);
    });

    let (app, cx) = cx.add_window_view(|window, cx| Jondo::boot(store, window, cx));
    cx.run_until_parked();
    app.update(cx, |app, cx| app.open_queue(cx));
    for _ in 0..3 {
        frame(cx);
        std::thread::sleep(std::time::Duration::from_millis(120));
    }
    frame(cx);

    let one = center(cx, "queue-row-0".to_owned());
    let two = center(cx, "queue-row-1".to_owned());

    cx.simulate_mouse_down(one, MouseButton::Left, Modifiers::default());
    cx.simulate_mouse_move(two, MouseButton::Left, Modifiers::default());
    frame(cx);
    assert!(
        app.read_with(cx, |app, _| app.dragging()),
        "moving over another row must start the drag"
    );

    cx.simulate_mouse_move(one, MouseButton::Left, Modifiers::default());
    frame(cx);
    cx.simulate_mouse_up(one, MouseButton::Left, Modifiers::default());
    frame(cx);

    assert!(
        !app.read_with(cx, |app, _| app.dragging()),
        "dropping a row back where it started must end the drag"
    );
}
