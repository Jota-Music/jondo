use gpui::{AnyElement, Div, IntoElement, ParentElement, Styled, div};

use crate::shared::ui::views::components::button;
use crate::shared::ui::views::design::theme;

/// Jota's `Loading`: a spinner centred in the space the view will fill. It
/// fills the box it is given and only falls back to a minimum height when the
/// parent has none, so a framed panel centres it instead of parking it on top.
pub fn spinner() -> Div {
    div()
        .size_full()
        .min_h(theme::unit(160.0))
        .flex()
        .items_center()
        .justify_center()
        .child(button::spinner(24.0, theme::MUTED))
}

/// Jota's `Rows`: the framed list shell with the spinner already inside it,
/// so the frame does not pop in once the rows land.
pub fn rows() -> AnyElement {
    div()
        .relative()
        .flex_1()
        .min_h(theme::unit(0.0))
        .child(
            div()
                .size_full()
                .rounded(theme::unit(8.0))
                .border_1()
                .border_color(theme::BORDER)
                .bg(theme::SURFACE)
                .child(spinner()),
        )
        .into_any_element()
}
