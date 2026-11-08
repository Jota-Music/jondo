use crate::shared::ui::views::design::theme;

use gpui::point;

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
