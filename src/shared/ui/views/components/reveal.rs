use crate::shared::ui::views::design::theme;

use gpui::{
    App, ClickEvent, Div, ElementId, InteractiveElement, IntoElement, MouseButton, ParentElement,
    Rgba, Stateful, StatefulInteractiveElement, Styled, Window, div,
};

use crate::shared::ui::views::design::icons::{self, Icon};

/// A control that stays hidden until the pointer enters its group.
pub fn show(hover: &str, inner: impl IntoElement) -> Div {
    div()
        .opacity(0.0)
        .group_hover(hover.to_owned(), |style| style.opacity(1.0))
        .child(inner)
}

/// The 32px overlay button a revealed control paints over artwork.
pub fn button(
    id: impl Into<ElementId>,
    icon: Icon,
    size: f32,
    tint: Rgba,
    background: Option<Rgba>,
    radius: f32,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let mut button = div()
        .id(id)
        .w(theme::unit(32.0))
        .h(theme::unit(32.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(theme::unit(radius))
        .cursor_pointer()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_click(on_click);
    if let Some(background) = background {
        button = button.bg(background);
    }

    button.child(icons::icon(icon, size, tint))
}
