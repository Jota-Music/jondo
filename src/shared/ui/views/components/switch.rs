use gpui::{App, Styled, Window};
use gpui_kit::component::FocusableExt as _;

pub fn switch(
    id: &'static str,
    checked: bool,
    color: gpui::Rgba,
    on_change: impl Fn(&bool, &mut Window, &mut App) + 'static,
) -> gpui_kit::component::switch::Switch {
    gpui_kit::component::switch::Switch::new(id)
        .checked(checked)
        .color(color)
        .focus_ring(false)
        .cursor_pointer()
        .on_change(on_change)
}
