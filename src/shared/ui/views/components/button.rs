use gpui::{
    App, Div, ElementId, InteractiveElement, IntoElement, ParentElement, Rgba, Role, SharedString,
    Stateful, StatefulInteractiveElement, Styled, Window, div,
};

use crate::shared::ui::views::components::hover;
use crate::shared::ui::views::components::paint::Scope;
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

/// The page reload. Same square and spinning state on every page that owns
/// one, so the control never changes place between sources.
pub fn reload(
    id: impl Into<ElementId>,
    refreshing: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    actionable(
        hover::hover_in(
            Scope::Both,
            id,
            true,
            move |fade| {
                div()
                    .w(theme::unit(40.0))
                    .h(theme::unit(40.0))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(theme::unit(6.0))
                    .child(if refreshing {
                        spinner(18.0, theme::mix(theme::MUTED, theme::TEXT, fade))
                            .into_any_element()
                    } else {
                        icons::icon(
                            icons::REFRESH_CW,
                            18.0,
                            theme::mix(theme::MUTED, theme::TEXT, fade),
                        )
                        .into_any_element()
                    })
            },
            on_click,
        ),
        t("Reload").to_owned().into(),
        true,
    )
}

/// The X that dismisses a top banner. Every banner carries the same one, so the
/// size, the ease and the label never drift between them.
pub fn dismiss(
    id: impl Into<ElementId>,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    dismiss_in(Scope::Both, id, on_click)
}

pub fn dismiss_in(
    scope: Scope,
    id: impl Into<ElementId>,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    hover::hover_in(
        scope,
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

/// An icon button that carries its hover in the glyph, not a wash behind it.
pub fn icon_flat(
    id: impl Into<ElementId>,
    icon: Icon,
    size: f32,
    color: Rgba,
    button: f32,
    enabled: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    icon_flat_in(
        Scope::Both,
        id,
        icon,
        size,
        color,
        button,
        enabled,
        on_click,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn icon_flat_in(
    scope: Scope,
    id: impl Into<ElementId>,
    icon: Icon,
    size: f32,
    color: Rgba,
    button: f32,
    enabled: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let label = icon.label();
    flat(
        scope,
        id,
        icon,
        size,
        color,
        button,
        enabled,
        label.to_owned().into(),
        on_click,
    )
}

/// The same button under a name of its own, for a glyph that stands for more
/// than one action: the reset arrow is not a reload.
#[allow(clippy::too_many_arguments)]
pub fn icon_named(
    id: impl Into<ElementId>,
    icon: Icon,
    size: f32,
    color: Rgba,
    button: f32,
    enabled: bool,
    label: &'static str,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    flat(
        Scope::Both,
        id,
        icon,
        size,
        color,
        button,
        enabled,
        label.into(),
        on_click,
    )
}

/// An icon that owns its square and fills it while hovered, for a button that
/// sits inside another surface: a stepper needs its own hit area to read as a
/// button, not as a glyph floating in the frame.
#[allow(clippy::too_many_arguments)]
pub fn tile(
    id: impl Into<ElementId>,
    icon: Icon,
    size: f32,
    color: Rgba,
    surface: Rgba,
    width: f32,
    enabled: bool,
    label: &'static str,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let named = label.to_owned().into();
    actionable(
        hover::hover_in(
            Scope::Both,
            id,
            enabled,
            move |fade| {
                div()
                    .w(theme::unit(width))
                    .h_full()
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .bg(theme::mix(surface, theme::ZINC_800, fade))
                    .child(icons::icon(
                        icon,
                        size,
                        theme::mix(color, theme::TEXT, fade),
                    ))
            },
            on_click,
        )
        .active(|style| style.bg(theme::ZINC_700)),
        named,
        enabled,
    )
    .tooltip(move |window, cx| Tooltip::new(label.to_owned()).build(window, cx))
}

#[allow(clippy::too_many_arguments)]
fn flat(
    scope: Scope,
    id: impl Into<ElementId>,
    icon: Icon,
    size: f32,
    color: Rgba,
    button: f32,
    enabled: bool,
    label: SharedString,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let named = label.clone();
    actionable(
        hover::hover_in(
            scope,
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
    .tooltip(move |window, cx| Tooltip::new(label.clone()).build(window, cx))
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
    circle_in(
        Scope::Both,
        id,
        icon,
        diameter,
        icon_size,
        fill,
        color,
        enabled,
        on_click,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn circle_in(
    scope: Scope,
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
    let named = label.to_owned().into();
    actionable(
        hover::hover_in(
            scope,
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
/// The lit state takes the cover's dominant colour, so the heart agrees with
/// the rest of the accent in the page.
pub fn heart(
    id: impl Into<ElementId>,
    on: bool,
    accent: Rgba,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> gpui::Stateful<Div> {
    // A finger needs the larger target the phone gives every toolbar button.
    let size = if super::narrow() { 40.0 } else { 32.0 };
    let glyph = 18.0;

    let (idle, lit) = if on {
        (accent, theme::mix(accent, theme::rgb(0xffffff), 0.35))
    } else {
        (theme::MUTED, theme::TEXT)
    };

    actionable(
        hover::hover_in(
            Scope::Both,
            id,
            true,
            move |fade| {
                div()
                    .w(theme::unit(size))
                    .h(theme::unit(size))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(theme::unit(6.0))
                    .child(icons::icon(
                        if on { icons::HEART_FILL } else { icons::HEART },
                        glyph,
                        theme::mix(idle, lit, fade),
                    ))
            },
            on_click,
        ),
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
