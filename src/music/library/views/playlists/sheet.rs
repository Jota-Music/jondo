use std::time::Instant;

use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Window, div,
};

use crate::shared::ui::views::components::text;
use crate::shared::ui::views::design::theme;
use crate::shell::app::Jondo;
use crate::shell::views::sheet;

#[allow(clippy::too_many_arguments)]
pub(super) fn sheet(
    app: &Jondo,
    id: &'static str,
    title: &str,
    title_size: f32,
    closing: Option<Instant>,
    bar: bool,
    window: &mut Window,
    cx: &mut Context<Jondo>,
    body: impl IntoElement,
) -> AnyElement {
    let content = div()
        .flex()
        .flex_col()
        .child(sheet::header(sheet::Header {
            title: Some(title.to_owned().into()),
            subtitle: None,
            icon: None,
            size: title_size,
            detail: None,
        }))
        .child(div().w_full().h(theme::unit(1.0)).bg(theme::BORDER))
        .child(body);

    // The picker binds this handle to its scrolling list, so a downward pan
    // only pulls the sheet once that list is back at the top.
    let drift = app.panes.get(id).cloned().unwrap_or_default();

    let sheet = sheet::Sheet::new(
        id,
        closing,
        |app, _, cx| {
            app.ui.closing = Some(Instant::now());
            cx.notify();
        },
        content,
    )
    .bar(bar)
    .at_top(move || drift.offset().y >= gpui::px(0.0));
    sheet.show(window, cx)
}

pub(super) fn accent_button(
    id: &'static str,
    label: &'static str,
    app: &Jondo,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    let fill = app.accent;
    let color = app.binary;
    div()
        .id(id)
        .debug_selector(move || id.to_owned())
        .h(theme::unit(40.0))
        .px(theme::unit(16.0))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(theme::unit(6.0))
        .bg(fill)
        .cursor_pointer()
        .occlude()
        .hover(move |style| style.bg(theme::mix(fill, theme::SURFACE, 0.25)))
        .on_click(on_click)
        .child(text::weighted(label, 14.0, color, theme::Weight::Medium))
}

pub(super) fn outline_button(
    id: &'static str,
    label: &'static str,
    _app: &Jondo,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .h(theme::unit(34.0))
        .px(theme::unit(16.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(theme::unit(6.0))
        .border_1()
        .border_color(theme::BORDER)
        .cursor_pointer()
        .occlude()
        .hover(|style| style.bg(theme::ZINC_900))
        .on_click(on_click)
        .child(text::text(label, 14.0, theme::TEXT))
}

pub(super) fn danger_button(
    id: &'static str,
    label: &'static str,
    _app: &Jondo,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .h(theme::unit(34.0))
        .px(theme::unit(16.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(theme::unit(6.0))
        .bg(theme::DANGER)
        .cursor_pointer()
        .occlude()
        .on_click(on_click)
        .child(text::weighted(
            label,
            14.0,
            theme::rgb(0x000000),
            theme::Weight::Medium,
        ))
}
