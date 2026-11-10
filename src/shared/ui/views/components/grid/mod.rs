use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, App, ClickEvent, Div, ElementId, InteractiveElement, IntoElement, ParentElement,
    ScrollHandle, Stateful, StatefulInteractiveElement, Styled, Window, div,
};

use crate::shared::ui::views::components::{hover, scroll};
use crate::shared::ui::views::design::{icons, theme};

/// The inner left inset and the scrollbar gutter every framed pane keeps.
pub const LEFT: f32 = 4.0;
pub const GUTTER: f32 = 15.0;
const BORDER: f32 = 2.0;

pub fn narrow(window: &Window) -> bool {
    window.viewport_size().width < theme::unit(768.0)
}

pub fn gap(window: &Window) -> f32 {
    if narrow(window) { 2.0 } else { 16.0 }
}

/// The square a card occupies in the 672px column, once the panel border and
/// the scroll gutter are removed. Every card grid must use this size or the
/// last column wraps under the others.
pub fn cell(window: &Window) -> f32 {
    cell_in(window, 3.0, gap(window))
}

/// The same square for a grid that lays out `columns` cards per row with
/// `column_gap` between them.
pub fn cell_in(window: &Window, columns: f32, column_gap: f32) -> f32 {
    let width = window.viewport_size().width.as_f32();
    cell_for(width, columns, column_gap)
}

fn cell_for(width: f32, columns: f32, column_gap: f32) -> f32 {
    let column = (width - if width < 768.0 { 32.0 } else { 0.0 }).min(theme::CONTENT_WIDTH);
    ((column - LEFT - GUTTER - BORDER - column_gap * (columns - 1.0)) / columns)
        .floor()
        .max(96.0)
}

pub fn show(
    id: &'static str,
    scroll: &ScrollHandle,
    window: &Window,
    tiles: Vec<AnyElement>,
) -> AnyElement {
    show_at(id, scroll, gap(window), tiles)
}

pub fn show_at(
    id: &'static str,
    scroll: &ScrollHandle,
    column_gap: f32,
    tiles: Vec<AnyElement>,
) -> AnyElement {
    pane(
        id,
        scroll,
        div()
            .flex()
            .flex_wrap()
            .gap(theme::unit(column_gap))
            .children(tiles)
            .into_any_element(),
    )
}

pub fn list(id: &'static str, scroll: &ScrollHandle, rows: Vec<AnyElement>) -> AnyElement {
    pane(
        id,
        scroll,
        div().flex().flex_col().children(rows).into_any_element(),
    )
}

pub fn toggle(
    id: &'static str,
    grid: bool,
    height: f32,
    on_grid: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_list: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    div()
        .flex()
        .rounded(theme::unit(6.0))
        .border_1()
        .border_color(theme::BORDER)
        .child(option(
            format!("{id}-grid"),
            icons::LAYOUT_GRID,
            grid,
            false,
            height,
            on_grid,
        ))
        .child(option(
            format!("{id}-list"),
            icons::LIST,
            !grid,
            true,
            height,
            on_list,
        ))
}

fn option(
    id: String,
    icon: icons::Icon,
    active: bool,
    divider: bool,
    height: f32,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let element = ElementId::Name(id.into());
    let label = icon.label();
    let width = if height > 32.0 { 40.0 } else { 32.0 };
    let option = hover::hover(
        element,
        true,
        move |fade| {
            div()
                .relative()
                .w(theme::unit(width))
                .h(theme::unit(height))
                .flex()
                .items_center()
                .justify_center()
                .when(active, |this| this.bg(theme::ZINC_800))
                .when(divider, |this| {
                    this.child(
                        div()
                            .absolute()
                            .left(theme::unit(0.0))
                            .top(theme::unit(0.0))
                            .bottom(theme::unit(0.0))
                            .w(theme::unit(1.0))
                            .bg(theme::BORDER),
                    )
                })
                .child(icons::icon(
                    icon,
                    14.0,
                    if active {
                        theme::rgb(0xffffff)
                    } else {
                        theme::mix(theme::FAINT, theme::rgb(0xffffff), fade)
                    },
                ))
        },
        on_click,
    );
    option.role(gpui::Role::Button).aria_label(label)
}

fn pane(id: &'static str, scroll: &ScrollHandle, body: AnyElement) -> AnyElement {
    div()
        .relative()
        .flex_1()
        .h_full()
        .min_h(theme::unit(0.0))
        .child(
            div()
                .id(id)
                .size_full()
                .overflow_y_scroll()
                .track_scroll(scroll)
                .pl(theme::unit(LEFT))
                .pr(theme::unit(GUTTER))
                .pb(theme::unit(12.0))
                .child(body),
        )
        .child(scroll::bar(scroll))
        .into_any_element()
}

#[cfg(test)]
#[path = "grid.test.rs"]
mod tests;
