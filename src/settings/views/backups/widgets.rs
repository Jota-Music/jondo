use gpui::prelude::FluentBuilder as _;
use gpui::{
    InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement, Styled, Window, div,
};

use crate::shared::kernel::app::clock::now_ms;
use crate::shared::ui::views::components::text;
use crate::shared::ui::views::design::icons::{self, Icon};
use crate::shared::ui::views::design::theme;
use crate::shell::app::Jondo;

pub(crate) fn heading(icon: Icon, title: &str) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .gap(theme::unit(8.0))
        .child(icons::icon(icon, 16.0, theme::MUTED))
        .child(text::weighted(
            title.to_owned(),
            12.0,
            theme::ZINC_200,
            theme::Weight::SemiBold,
        ))
}

pub(crate) fn row(left: gpui::Div, right: impl IntoElement) -> gpui::Div {
    div()
        .rounded(theme::unit(12.0))
        .border_1()
        .border_color(theme::ZINC_800)
        .bg(theme::SURFACE)
        .p(theme::unit(12.0))
        .child(line(left, right))
}

pub(crate) fn line(left: gpui::Div, right: impl IntoElement) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(theme::unit(12.0))
        .child(left)
        .child(right)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn action(
    id: &'static str,
    label: &'static str,
    icon: Icon,
    accent: bool,
    full: bool,
    enabled: bool,
    app: &Jondo,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    let fill = if accent { app.accent } else { theme::SURFACE };
    let color = if accent { app.binary } else { theme::ZINC_100 };
    let border = if accent { app.accent } else { theme::ZINC_700 };
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
        .border_1()
        .border_color(border)
        .bg(fill)
        .role(gpui::Role::Button)
        .aria_label(label)
        .when(enabled, |this| {
            this.cursor_pointer().hover(move |style| {
                if accent {
                    style.opacity(0.85)
                } else {
                    style.border_color(theme::FAINT)
                }
            })
        })
        .when(!enabled, |this| this.opacity(0.5).cursor_default())
        .when(enabled, |this| this.on_click(on_click))
        .child(icons::icon(icon, 12.0, color))
        .child(text::weighted(label, 12.0, color, theme::Weight::SemiBold))
}

pub(crate) fn stat(label: &str, value: String, color: gpui::Rgba) -> gpui::Div {
    div()
        .flex()
        .items_baseline()
        .justify_between()
        .gap(theme::unit(12.0))
        .child(text::text(label.to_owned(), 11.0, theme::FAINT))
        .child(text::numeric(value, 11.0, color))
}

pub(crate) fn ago(at: i64) -> String {
    let minutes = ((now_ms() - at).max(0) / 60_000) as u64;
    if minutes < 60 {
        format!("{minutes} min")
    } else if minutes < 60 * 24 {
        format!("{} h", minutes / 60)
    } else {
        format!("{} d", minutes / (60 * 24))
    }
}

pub(crate) fn size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{} KB", (bytes as f64 / 1024.0).round() as usize)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}
