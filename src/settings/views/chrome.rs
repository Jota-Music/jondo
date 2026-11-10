use crate::shared::ui::views::components::{button, surface, text};
use crate::shared::ui::views::design::icons::{self, Icon};
use crate::shared::ui::views::design::theme;
use crate::shell::app::Jondo;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, App, Div, InteractiveElement, IntoElement, ParentElement, SharedString, Stateful,
    StatefulInteractiveElement, Styled, Window, div,
};

pub(super) fn section(
    icon: Icon,
    title: &str,
    action: Option<AnyElement>,
    body: impl IntoElement,
) -> AnyElement {
    let mut header = div()
        .flex()
        .items_center()
        .justify_between()
        .gap(theme::unit(8.0))
        .child(
            div()
                .flex()
                .items_center()
                .gap(theme::unit(8.0))
                .child(icons::icon(icon, 18.0, theme::MUTED))
                .child(text::weighted(
                    title.to_owned(),
                    14.0,
                    theme::ZINC_200,
                    theme::Weight::SemiBold,
                )),
        );
    if let Some(action) = action {
        header = header.child(action);
    }

    surface(
        16.0,
        theme::mix(theme::BG, theme::ZINC_900, 0.4),
        theme::ZINC_800,
    )
    .p(theme::unit(16.0))
    .flex()
    .flex_col()
    .gap(theme::unit(16.0))
    .child(header)
    .child(body)
    .into_any_element()
}

pub(super) fn card(inner: impl IntoElement) -> Div {
    surface(12.0, theme::SURFACE, theme::ZINC_800)
        .p(theme::unit(12.0))
        .child(inner)
}

pub(super) fn accent_button(
    id: &'static str,
    label: &'static str,
    full: bool,
    app: &Jondo,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let fill = app.accent;
    let color = app.binary;
    div()
        .id(id)
        .when(full, |this| this.w_full())
        .h(theme::unit(32.0))
        .px(theme::unit(12.0))
        .flex()
        .items_center()
        .justify_center()
        .gap(theme::unit(6.0))
        .rounded(theme::unit(8.0))
        .bg(fill)
        .cursor_pointer()
        .on_click(on_click)
        .child(text::weighted(label, 12.0, color, theme::Weight::SemiBold))
}

pub(super) fn danger_button(
    id: &'static str,
    label: &'static str,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .h(theme::unit(32.0))
        .px(theme::unit(12.0))
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .rounded(theme::unit(8.0))
        .bg(theme::alpha(theme::rgb(0x7f1d1d), 0.5))
        .cursor_pointer()
        .on_click(on_click)
        .child(text::weighted(
            label,
            12.0,
            theme::rgb(0xfecaca),
            theme::Weight::SemiBold,
        ))
}

pub(super) fn filled_button(
    id: &'static str,
    label: impl Into<SharedString>,
    loading: bool,
    enabled: bool,
    full: bool,
    large: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let height = if large { 40.0 } else { 32.0 };
    let padding = if large { 16.0 } else { 12.0 };
    let gap = if large { 8.0 } else { 6.0 };
    let size = if large { 14.0 } else { 12.0 };
    div()
        .id(id)
        .when(full, |this| this.w_full())
        .h(theme::unit(height))
        .px(theme::unit(padding))
        .flex()
        .items_center()
        .justify_center()
        .gap(theme::unit(gap))
        .rounded(theme::unit(8.0))
        .bg(theme::ZINC_800)
        .on_click(on_click)
        .when(enabled, |this| {
            this.cursor_pointer()
                .hover(|style| style.bg(theme::ZINC_700))
        })
        .when(!enabled, |this| this.opacity(0.5).cursor_default())
        .when(loading, |this| {
            this.child(button::spinner(14.0, theme::ZINC_100))
        })
        .child(text::weighted(
            label,
            size,
            theme::ZINC_100,
            theme::Weight::SemiBold,
        ))
}
