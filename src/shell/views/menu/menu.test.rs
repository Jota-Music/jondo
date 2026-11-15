use crate::shared::music::domain::Song;
use crate::shared::storage::{LocalPlaylist, Store};
use crate::shared::ui::views::design::theme;
use crate::shell::app::{Jondo, Route};

use gpui::{ElementId, MouseButton, TestAppContext, VisualTestContext, point};

use super::view::{presents_as_sheet, toggle};
use super::{Command, Item, Menu};

fn menu(checked: Option<usize>) -> Menu {
    let items = (0..3)
        .map(|index| Item::choice(format!("{index}"), checked == Some(index), Command::Toggle))
        .collect();
    Menu::choice(point(theme::unit(0.0), theme::unit(0.0)), "Sort", items)
}

#[test]
fn the_highlight_starts_on_the_current_value() {
    assert_eq!(menu(Some(1)).index, Some(1));
    assert_eq!(menu(None).index, None);
}

#[test]
fn arrows_move_without_wrapping() {
    let mut menu = menu(None);
    menu.step(1);
    assert_eq!(menu.index, Some(0));
    menu.step(-1);
    assert_eq!(menu.index, Some(0));
    menu.step(1);
    menu.step(1);
    menu.step(1);
    assert_eq!(menu.index, Some(2));
}

#[test]
fn context_menus_are_modal_only_on_android() {
    assert!(presents_as_sheet(true, false, true));
    assert!(!presents_as_sheet(true, true, false));
    assert!(presents_as_sheet(false, true, false));
    assert!(!presents_as_sheet(false, false, true));
}

#[gpui::test]
fn the_trigger_that_opened_a_menu_closes_it_on_the_next_press(cx: &mut TestAppContext) {
    let home = std::env::temp_dir().join(format!("jondo-menu-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&home);
    crate::shared::storage::use_home(&home);
    cx.update(|cx| {
        gpui_kit::init(cx);
        theme::install(cx);
    });
    let (app, cx) = cx.add_window_view(|window, cx| {
        crate::shell::app::Jondo::boot(crate::shared::storage::Store::load(), window, cx)
    });
    cx.run_until_parked();

    app.update(cx, |app, _| {
        let origin = ElementId::Name("trigger".into());
        let menu = || menu(None);

        assert!(toggle(app, origin.clone(), menu()));
        assert!(app.menu.as_ref().is_some_and(|menu| menu.from(&origin)));

        assert!(!toggle(app, origin.clone(), menu()));
        assert!(app.menu_closing.is_some());

        // A press while the menu slides out opens it again instead of closing.
        assert!(toggle(app, origin.clone(), menu()));
        assert!(app.menu_closing.is_none());
        assert!(app.menu.as_ref().is_some_and(|menu| menu.from(&origin)));

        // A different trigger replaces the open menu rather than closing it.
        let other = ElementId::Name("other".into());
        assert!(toggle(app, other.clone(), menu()));
        assert!(app.menu.as_ref().is_some_and(|menu| menu.from(&other)));
    });
}

fn frame(cx: &mut VisualTestContext) {
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
}

/// A press on one trigger, then on another, must move the menu with a single
/// click: the release of the second press opens it, it is not swallowed as the
/// dismissal of the first one.
#[gpui::test]
fn a_press_on_another_trigger_replaces_the_menu_in_one_click(cx: &mut TestAppContext) {
    let home = std::env::temp_dir().join(format!("jondo-menu-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&home);
    crate::shared::storage::use_home(&home);
    cx.update(|cx| {
        gpui_kit::init(cx);
        theme::install(cx);
    });

    let mut store = Store::load();
    store.playlists.push(LocalPlaylist {
        id: "local:mix".to_owned(),
        name: "Mix".to_owned(),
        songs: ["one", "two"]
            .map(|name| Song {
                id: format!("local:song:{name}"),
                name: name.to_owned(),
                duration: 120,
                ..Default::default()
            })
            .to_vec(),
        modified: 0,
    });

    let (app, cx) = cx.add_window_view(|window, cx| Jondo::boot(store, window, cx));
    cx.run_until_parked();
    app.update(cx, |app, cx| {
        app.open(Route::Playlist("local:mix".to_owned()), cx);
        cx.notify();
    });
    cx.run_until_parked();
    frame(cx);

    let first = cx
        .debug_bounds("song-actions-local:mix-0")
        .expect("the first row's trigger paints")
        .center();
    let second = cx
        .debug_bounds("song-actions-local:mix-1")
        .expect("the second row's trigger paints")
        .center();

    let origin = |cx: &mut VisualTestContext| {
        app.update(cx, |app, _| {
            app.menu.as_ref().and_then(|menu| menu.origin.clone())
        })
    };

    cx.simulate_click(first, Default::default());
    frame(cx);
    assert_eq!(
        origin(cx),
        Some(ElementId::Name("song-actions-local:mix-0".into()))
    );

    cx.simulate_click(second, Default::default());
    frame(cx);
    assert_eq!(
        origin(cx),
        Some(ElementId::Name("song-actions-local:mix-1".into())),
        "the second press must open its own menu, not dismiss the first one"
    );

    // A right press opens its menu before the pointer is released, and a frame
    // may paint it in between: that release belongs to the opening press, so it
    // must not dismiss the menu it has just opened.
    let row = cx
        .debug_bounds("song-row-0")
        .expect("the first row paints")
        .center();
    cx.simulate_mouse_down(row, MouseButton::Right, Default::default());
    frame(cx);
    cx.simulate_mouse_up(row, MouseButton::Right, Default::default());
    frame(cx);
    assert!(
        app.update(cx, |app, _| app.menu.is_some()
            && app.menu_closing.is_none()),
        "the release of the press that opened a menu keeps it open"
    );

    cx.simulate_click(point(gpui::px(4.0), gpui::px(4.0)), Default::default());
    assert!(
        app.update(cx, |app, _| app.menu.is_none()
            || app.menu_closing.is_some()),
        "a press away from the panel dismisses the open menu"
    );
}
