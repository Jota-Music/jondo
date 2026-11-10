use gpui::{Div, IntoElement, ParentElement, Styled, div};

use crate::shared::ui::views::design::theme;

use super::surface;

/// The 672px column the application centers on every screen.
pub fn page(inner: impl IntoElement) -> Div {
    div().size_full().flex().justify_center().child(
        div()
            .w(theme::unit(theme::CONTENT_WIDTH))
            .h_full()
            .max_w_full()
            .min_w(theme::unit(0.0))
            .child(inner),
    )
}

/// The same 672px column without forcing the height, for panels inside a column.
pub fn column(inner: impl IntoElement) -> Div {
    div().w_full().flex().justify_center().child(
        div()
            .w(theme::unit(theme::CONTENT_WIDTH))
            .max_w_full()
            .min_w(theme::unit(0.0))
            .child(inner),
    )
}

/// The surface a page or panel paints its content on.
pub fn panel(inner: impl IntoElement) -> Div {
    surface(8.0, theme::SURFACE, theme::BORDER)
        .flex_1()
        .min_h(theme::unit(0.0))
        .w_full()
        .overflow_hidden()
        .child(inner)
}
