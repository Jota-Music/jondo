use gpui::{
    App, Div, ElementId, InteractiveElement, IntoElement, ParentElement, Rgba, Role, SharedString,
    Stateful, StatefulInteractiveElement, Styled, Window, div,
};

use crate::shared::ui::views::components::hover;
use crate::shared::ui::views::design::icons::{self, Icon};
use crate::shared::ui::views::design::theme;
use gpui_kit::component::tooltip::Tooltip;

use crate::shared::i18n::t;

/// A div is not a button until it says so: give it a role, a name and, when it
/// can act, a tab stop and the same Enter/Space activation a press gets.
fn actionable(element: Stateful<Div>, label: SharedString, enabled: bool) -> Stateful<Div> {
    let element = element.role(Role::Button).aria_label(label);
    if enabled {
        element
            .focusable()
            .tab_stop(true)
            .focus_visible(|style| style.opacity(0.7))
    } else {
        element
    }
}

pub fn icon_button(
    id: impl Into<ElementId>,
    icon: Icon,
    size: f32,
    color: Rgba,
    button: f32,
    enabled: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let base = theme::alpha(theme::ZINC_800, 0.0);
    let lit = theme::alpha(theme::ZINC_800, 0.8);
    let label = icon.label();
    let named: SharedString = label.to_owned().into();
    actionable(
        hover::hover(
            id,
            enabled,
            move |fade| {
                div()
                    .w(theme::unit(button))
                    .h(theme::unit(button))
                    .flex()
                    .flex_none()
                    .items_center()
                    .justify_center()
                    .rounded(theme::unit(6.0))
                    .cursor_pointer()
                    .bg(theme::mix(base, lit, fade))
                    .child(icons::icon(
                        icon,
                        size,
                        theme::mix(color, theme::TEXT, fade),
                    ))
            },
            on_click,
        ),
        named,
        enabled,
    )
    .tooltip(move |window, cx| Tooltip::new(label).build(window, cx))
}

/// The page reload. Same square, border and spinning state on every page that
/// owns one, so the control never changes place between sources.
pub fn reload(
    id: impl Into<ElementId>,
    refreshing: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    actionable(
        div()
            .id(id)
            .w(theme::unit(40.0))
            .h(theme::unit(40.0))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .rounded(theme::unit(6.0))
            .border_1()
            .border_color(theme::BORDER)
            .bg(theme::SURFACE)
            .cursor_pointer()
            .on_click(on_click),
        t("Reload").to_owned().into(),
        true,
    )
    .child(if refreshing {
        spinner(16.0, theme::MUTED).into_any_element()
    } else {
        icons::icon(icons::REFRESH_CW, 16.0, theme::MUTED).into_any_element()
    })
}

/// The X that dismisses a top banner. Every banner carries the same one, so the
/// size, the ease and the label never drift between them.
pub fn dismiss(
    id: impl Into<ElementId>,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    hover::hover(
        id,
        true,
        |fade| {
            div()
                .w(theme::unit(24.0))
                .h(theme::unit(24.0))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .child(icons::icon(
                    icons::X,
                    14.0,
                    theme::mix(theme::FAINT, theme::ZINC_200, fade),
                ))
        },
        on_click,
    )
    .tooltip(|window, cx| Tooltip::new(t("Dismiss")).build(window, cx))
}

/// An icon with no hover surface of its own, for bars and rows that own the state.
pub fn icon_flat(
    id: impl Into<ElementId>,
    icon: Icon,
    size: f32,
    color: Rgba,
    button: f32,
    enabled: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let label = icon.label();
    let named: SharedString = label.to_owned().into();
    actionable(
        hover::hover(
            id,
            enabled,
            move |fade| {
                div()
                    .w(theme::unit(button))
                    .h(theme::unit(button))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .child(icons::icon(
                        icon,
                        size,
                        theme::mix(color, theme::TEXT, fade),
                    ))
            },
            on_click,
        ),
        named,
        enabled,
    )
    .tooltip(move |window, cx| Tooltip::new(label).build(window, cx))
}

#[allow(clippy::too_many_arguments)]
pub fn circle(
    id: impl Into<ElementId>,
    icon: Icon,
    diameter: f32,
    icon_size: f32,
    fill: Rgba,
    color: Rgba,
    enabled: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let label = icon.label();
    let named: SharedString = label.to_owned().into();
    actionable(
        hover::hover(
            id,
            enabled,
            move |fade| {
                div()
                    .w(theme::unit(diameter))
                    .h(theme::unit(diameter))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_full()
                    .bg(theme::shade(fill, 1.0 + 0.08 * fade))
                    .cursor_pointer()
                    .child(icons::icon(icon, icon_size, color))
            },
            on_click,
        ),
        named,
        enabled,
    )
    .tooltip(move |window, cx| Tooltip::new(label).build(window, cx))
}

pub fn circle_spinner(diameter: f32, icon_size: f32, fill: Rgba, color: Rgba) -> Div {
    div()
        .w(theme::unit(diameter))
        .h(theme::unit(diameter))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .bg(fill)
        .child(icons::spinning(icons::LOADER, icon_size, color))
}

/// The 32px heart every page header and user header uses to save or follow.
pub fn heart(
    id: impl Into<ElementId>,
    on: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> gpui::Stateful<Div> {
    actionable(
        div()
            .id(id)
            .w(theme::unit(32.0))
            .h(theme::unit(32.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded(theme::unit(6.0))
            .border_1()
            .border_color(theme::ZINC_800)
            .bg(theme::SURFACE)
            .cursor_pointer()
            .on_click(on_click)
            .child(icons::icon(
                if on { icons::HEART_FILL } else { icons::HEART },
                14.0,
                if on { theme::DANGER } else { theme::MUTED },
            )),
        if on { t("Unsave") } else { t("Save") }.to_owned().into(),
        true,
    )
}

pub fn spinner(size: f32, color: Rgba) -> impl IntoElement {
    icons::spinning(icons::LOADER, size, color)
}

#[cfg(test)]
#[path = "button.test.rs"]
mod tests;
