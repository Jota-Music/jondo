use gpui::prelude::FluentBuilder as _;
use gpui::{
    App, Div, ElementId, Entity, InteractiveElement, MouseButton, ParentElement, Stateful,
    StatefulInteractiveElement, Styled, div,
};
use gpui_kit::component::input::{Input, InputState};

use crate::shared::i18n::t;
use crate::shared::ui::views::design::{icons, theme};

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
    boxed(state, false, false, None)
}

/// A field the value pins from the environment cannot be edited in.
pub fn disabled(state: &Entity<InputState>, disabled: bool) -> Div {
    boxed(state, disabled, false, None)
}

/// A password field: the value is drawn as dots and the eye reveals it.
pub fn secret(state: &Entity<InputState>, cx: &App) -> Div {
    boxed(state, false, true, Some(cx))
}

/// A password field the value pins from the environment cannot be edited in.
pub fn secret_disabled(state: &Entity<InputState>, disabled: bool, cx: &App) -> Div {
    boxed(state, disabled, true, Some(cx))
}

fn boxed(state: &Entity<InputState>, disabled: bool, masked: bool, cx: Option<&App>) -> Div {
    let input = input(state).disabled(disabled);
    let input = if masked {
        input.flex_1().min_w(theme::unit(0.0))
    } else {
        input
    };
    let eye = cx.map(|cx| reveal(state, cx));
    div()
        .h(theme::unit(40.0))
        .rounded(theme::unit(6.0))
        .border_1()
        .border_color(theme::BORDER)
        .bg(theme::SURFACE)
        .px(theme::unit(12.0))
        .flex()
        .items_center()
        .block_mouse_except_scroll()
        .when(disabled, |this| this.opacity(0.5))
        .child(input)
        .children(eye)
}

/// The eye that flips a secret field. The 16px glyph sits on a 32px surface so
/// a thumb finds it, and a tap keeps the caret where it was instead of dropping
/// the keyboard.
pub fn reveal(state: &Entity<InputState>, cx: &App) -> Stateful<Div> {
    let masked = state.read(cx).presentation().is_masked();
    let target = state.clone();
    div()
        .id(ElementId::NamedInteger(
            "secret-reveal".into(),
            state.entity_id().as_u64(),
        ))
        .w(theme::unit(32.0))
        .h(theme::unit(32.0))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(theme::unit(6.0))
        .cursor_pointer()
        .hover(|style| style.bg(theme::ZINC_800))
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_click(move |_, window, cx| {
            target.update(cx, |state, cx| {
                state.toggle_masked(window, cx);
                state.focus(window, cx);
            });
        })
        .role(gpui::Role::Button)
        .debug_selector(|| "secret-reveal".to_owned())
        .aria_label(if masked {
            t("Show password")
        } else {
            t("Hide password")
        })
        .child(icons::icon(
            if masked { icons::EYE } else { icons::EYE_OFF },
            16.0,
            theme::FAINT,
        ))
}

#[cfg(test)]
#[path = "field.test.rs"]
mod tests;
