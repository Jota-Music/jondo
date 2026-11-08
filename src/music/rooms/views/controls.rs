use gpui::prelude::FluentBuilder as _;
use gpui::{
    App, ClickEvent, Div, ElementId, InteractiveElement, ParentElement, SharedString, Stateful,
    StatefulInteractiveElement, Styled, Window, div,
};

use crate::shared::ui::views::components::text;
use crate::shared::ui::views::design::icons::{self, Icon};
use crate::shared::ui::views::design::theme;

// The room actions follow Jota's session panel: one full-width row per action,
// an icon on the left of its label, and no pill shapes.

enum Tone {
    Filled,
    Outline,
    Danger,
}

pub(super) fn action(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    icon: Icon,
    disabled: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    button(id, label, icon, Tone::Filled, true, disabled, on_click)
}

pub(super) fn outline(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    icon: Icon,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    button(id, label, icon, Tone::Outline, true, false, on_click)
}

pub(super) fn danger(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    icon: Icon,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    button(id, label, icon, Tone::Danger, true, false, on_click)
}

pub(super) fn inline(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    icon: Icon,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    button(id, label, icon, Tone::Filled, false, false, on_click)
}

pub(super) fn compact(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    icon: Icon,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    button(id, label, icon, Tone::Outline, false, false, on_click)
}

fn button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    icon: Icon,
    tone: Tone,
    full: bool,
    disabled: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let mut element = div()
        .id(id)
        .h(theme::unit(40.0))
        .px(theme::unit(16.0))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .gap(theme::unit(8.0))
        .rounded(theme::unit(8.0))
        .occlude();
    if !disabled {
        element = element.cursor_pointer();
    }

    if full {
        element = element.w_full();
    }

    let (fill, text, border, hover) = match tone {
        Tone::Filled => (
            Some(theme::ZINC_800),
            theme::ZINC_100,
            None,
            Some(theme::ZINC_700),
        ),
        Tone::Outline => (
            None,
            theme::ZINC_300,
            Some(theme::BORDER),
            Some(theme::ZINC_800),
        ),
        Tone::Danger => (
            Some(theme::alpha(theme::rgb(0x7f1d1d), 0.4)),
            theme::rgb(0xfecaca),
            None,
            Some(theme::alpha(theme::rgb(0x7f1d1d), 0.6)),
        ),
    };

    if let Some(fill) = fill {
        element = element.bg(fill);
    }

    if let Some(border) = border {
        element = element.border_1().border_color(border);
    }

    if !disabled && let Some(hover) = hover {
        element = element.hover(move |style| style.bg(hover));
    }

    element
        .when(disabled, |this| this.opacity(0.4))
        .on_click(move |event, window, cx| {
            if !disabled {
                on_click(event, window, cx);
            }
        })
        .child(icons::icon(icon, 16.0, text))
        .child(text::text(label, 14.0, text))
}
