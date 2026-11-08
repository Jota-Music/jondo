use gpui::{
    App, ClickEvent, Div, InteractiveElement, ParentElement, Rgba, Stateful,
    StatefulInteractiveElement, Styled, Window, div,
};

use crate::shared::ui::views::components::text;
use crate::shared::ui::views::design::icons::{self, Icon};
use crate::shared::ui::views::design::theme;

/// A control that opens a `Menu::choice` on click.
pub struct Select {
    pub id: &'static str,
    pub title: &'static str,
    pub label: String,
    pub open: bool,
    pub icon: Option<Icon>,
    pub chevron: bool,
    pub full: bool,
    pub radius: f32,
    pub border: Rgba,
}

pub fn show(
    select: Select,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let mut row = div()
        .id(select.id)
        .h(theme::unit(40.0))
        .flex()
        .items_center()
        .rounded(theme::unit(select.radius))
        .border_1()
        .border_color(select.border)
        .bg(theme::SURFACE)
        .cursor_pointer()
        .hover(|style| style.bg(theme::ZINC_900))
        .role(gpui::Role::Button)
        .aria_expanded(select.open)
        .aria_label(format!("{}: {}", select.title, select.label))
        .on_click(on_click);
    row = if select.chevron {
        row.pl(theme::unit(12.0))
            .pr(theme::unit(8.0))
            .justify_between()
    } else {
        row.px(theme::unit(12.0)).gap(theme::unit(8.0))
    };

    if select.full {
        row = row.w_full();
    }

    if select.open {
        row = row.bg(theme::ZINC_900).border_color(theme::ZINC_600);
    }

    row.children(
        select
            .icon
            .map(|icon| icons::icon(icon, 16.0, theme::FAINT)),
    )
    .child(text::text(select.label, 14.0, theme::TEXT))
    .children(
        select
            .chevron
            .then(|| icons::icon(icons::CHEVRON_DOWN, 16.0, theme::MUTED)),
    )
}
