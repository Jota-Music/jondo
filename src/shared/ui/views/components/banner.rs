use gpui::{Div, Styled, div};

use crate::shared::ui::views::design::theme;

/// The shell every top banner shares: full width, one row, the same padding,
/// gap and separator, so the clipboard, selection and update banners line up.
pub fn banner() -> Div {
    div()
        .w_full()
        .flex_none()
        .flex()
        .items_center()
        .gap(theme::unit(12.0))
        .px(theme::unit(16.0))
        .py(theme::unit(8.0))
        .bg(theme::mix(theme::BG, theme::ZINC_900, 0.6))
        .border_b_1()
        .border_color(theme::ZINC_800)
}
