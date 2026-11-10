use crate::shared::ui::views::design::theme;

use gpui::{ElementId, TestAppContext, point};

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
