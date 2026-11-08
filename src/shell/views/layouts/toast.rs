use gpui::{AnyElement, IntoElement, ParentElement, Styled, div};

use crate::shared::ui::views::design::theme;
use crate::shell::app::Jondo;

pub(super) fn toast(app: &Jondo, narrow: bool) -> AnyElement {
    let Some(toast) = &app.toast else {
        return div().into_any_element();
    };

    let age = toast.at.elapsed().as_secs_f32();
    let alpha = if age < 0.2 {
        age / 0.2
    } else if age > 3.8 {
        ((4.0 - age) / 0.2).clamp(0.0, 1.0)
    } else {
        1.0
    };

    let offset = if narrow {
        150.0
    } else if app.ui.compact {
        98.0
    } else {
        18.0
    };

    let color = if toast.error {
        crate::shared::ui::views::design::theme::DANGER
    } else {
        crate::shared::ui::views::design::theme::TEXT
    };

    div()
        .absolute()
        .left(theme::unit(0.0))
        .right(theme::unit(0.0))
        .bottom(theme::unit(offset))
        .flex()
        .justify_center()
        .opacity(alpha)
        .child(
            div()
                .max_w(theme::unit(theme::CONTENT_WIDTH))
                .mx(theme::unit(16.0))
                .px(theme::unit(14.0))
                .py(theme::unit(8.0))
                .rounded(theme::unit(8.0))
                .bg(theme::SURFACE)
                .border_1()
                .border_color(if toast.error {
                    theme::DANGER
                } else {
                    theme::BORDER
                })
                .child(crate::shared::ui::views::components::text::text(
                    toast.text.clone(),
                    12.0,
                    color,
                )),
        )
        .into_any_element()
}
