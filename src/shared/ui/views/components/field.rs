use gpui::prelude::FluentBuilder as _;
use gpui::{Div, Entity, InteractiveElement, ParentElement, Styled, div};
use gpui_kit::component::input::{Input, InputState};

use crate::shared::ui::views::design::theme;

/// The text input the application draws inside its own box.
pub fn input(state: &Entity<InputState>) -> Input {
    Input::new(state)
        .appearance(false)
        .bordered(false)
        .focus_bordered(false)
        .w_full()
        .px(theme::unit(0.0))
        .text_size(theme::unit(14.0))
}

/// The boxed field used by toolbars, dialogs and settings.
pub fn text(state: &Entity<InputState>) -> Div {
    boxed(state, false, false)
}

/// A field the value pins from the environment cannot be edited in.
pub fn disabled(state: &Entity<InputState>, disabled: bool) -> Div {
    boxed(state, disabled, false)
}

/// A password field: the value is drawn as dots and the eye reveals it.
pub fn secret(state: &Entity<InputState>) -> Div {
    boxed(state, false, true)
}

/// A password field the value pins from the environment cannot be edited in.
pub fn secret_disabled(state: &Entity<InputState>, disabled: bool) -> Div {
    boxed(state, disabled, true)
}

fn boxed(state: &Entity<InputState>, disabled: bool, masked: bool) -> Div {
    let input = input(state).disabled(disabled);
    div()
        .h(theme::unit(40.0))
        .rounded(theme::unit(6.0))
        .border_1()
        .border_color(theme::BORDER)
        .bg(theme::SURFACE)
        .px(theme::unit(12.0))
        .flex()
        .items_center()
        .occlude()
        .when(disabled, |this| this.opacity(0.5))
        .child(if masked { input.mask_toggle() } else { input })
}
