use gpui::{
    App, BoxShadow, Div, ElementId, InteractiveElement, IntoElement, ParentElement, Rgba, Stateful,
    StatefulInteractiveElement, Styled, Window, div, relative,
};

use crate::shared::ui::views::components::text;
use crate::shared::ui::views::design::icons::{self, Icon};
use crate::shared::ui::views::design::theme;

pub struct Hint<'a> {
    pub icon: Icon,
    pub tint: Rgba,
    pub title: &'a str,
    pub body: &'a str,
    pub steps: &'a [&'a str],
}

pub fn hint(hint: Hint<'_>, action: Option<Stateful<Div>>) -> Div {
    let tint = hint.tint;
    div()
        .size_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .p(theme::unit(32.0))
        .child(
            div()
                .relative()
                .w(theme::unit(384.0))
                .max_w_full()
                .flex()
                .flex_col()
                .items_center()
                .gap(theme::unit(20.0))
                .px(theme::unit(24.0))
                .py(theme::unit(32.0))
                .child(glow(tint))
                .child(badge(hint.icon, tint))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(theme::unit(4.0))
                        .child(
                            text::weighted(
                                hint.title,
                                14.0,
                                theme::ZINC_100,
                                theme::Weight::SemiBold,
                            )
                            .text_center(),
                        )
                        .child(text::text(hint.body, 12.0, theme::FAINT).text_center()),
                )
                .children((!hint.steps.is_empty()).then(|| steps(hint.steps)))
                .children(action),
        )
}

pub fn plain(message: &str) -> Div {
    div()
        .size_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .p(theme::unit(32.0))
        .child(text::text(message.to_owned(), 14.0, theme::FAINT).text_center())
}

/// The empty message a framed list centres while it has nothing to show.
pub fn muted(message: &str) -> Div {
    div()
        .size_full()
        .min_h(theme::unit(160.0))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .p(theme::unit(32.0))
        .child(text::text(message.to_owned(), 14.0, theme::MUTED))
}

/// The same centring without the floor, for a message inside a sized panel.
pub fn message(content: impl IntoElement) -> Div {
    div()
        .flex_1()
        .min_h(theme::unit(0.0))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .p(theme::unit(16.0))
        .child(content)
}

fn glow(tint: Rgba) -> Div {
    div()
        .absolute()
        .top(theme::unit(0.0))
        .left(theme::unit(0.0))
        .right(theme::unit(0.0))
        .flex()
        .justify_center()
        .child(
            div()
                .w(theme::unit(128.0))
                .h(theme::unit(128.0))
                .rounded_full()
                .shadow(vec![
                    BoxShadow::new(
                        theme::unit(0.0),
                        theme::unit(0.0),
                        theme::alpha(tint, 0.35).into(),
                    )
                    .blur_radius(theme::unit(64.0)),
                ]),
        )
}

fn badge(icon: Icon, tint: Rgba) -> Div {
    div()
        .w(theme::unit(48.0))
        .h(theme::unit(48.0))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .bg(theme::alpha(tint, 0.10))
        .border_1()
        .border_color(theme::alpha(tint, 0.25))
        .child(icons::icon(icon, 24.0, tint))
}

fn steps(steps: &[&str]) -> Div {
    let mut column = div().w_full().flex().flex_col();
    for (index, step) in steps.iter().enumerate() {
        let last = index + 1 == steps.len();
        let mut row = div()
            .relative()
            .w_full()
            .flex()
            .gap(theme::unit(12.0))
            .child(number(index))
            .child(
                text::text(*step, 12.0, theme::MUTED)
                    .line_height(relative(1.375))
                    .pt(theme::unit(2.0))
                    .flex_1()
                    .min_w(theme::unit(0.0)),
            );
        if last {
            row = row.pb(theme::unit(0.0));
        } else {
            row = row.pb(theme::unit(16.0)).child(
                div()
                    .absolute()
                    .left(theme::unit(12.0))
                    .top(theme::unit(28.0))
                    .bottom(theme::unit(0.0))
                    .w(theme::unit(1.0))
                    .bg(theme::ZINC_800),
            );
        }

        column = column.child(row);
    }

    column
}

fn number(index: usize) -> Div {
    div()
        .w(theme::unit(24.0))
        .h(theme::unit(24.0))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .border_1()
        .border_color(theme::ZINC_700)
        .bg(theme::ZINC_900)
        .child(text::weighted(
            (index + 1).to_string(),
            11.0,
            theme::ZINC_300,
            theme::Weight::SemiBold,
        ))
}

pub fn action(
    id: impl Into<ElementId>,
    label: &str,
    icon: Option<Icon>,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    div()
        .id(id)
        .h(theme::unit(40.0))
        .px(theme::unit(16.0))
        .flex()
        .items_center()
        .justify_center()
        .gap(theme::unit(8.0))
        .rounded(theme::unit(8.0))
        .bg(theme::ZINC_800)
        .cursor_pointer()
        .hover(|style| style.bg(theme::ZINC_700))
        .on_click(on_click)
        .children(icon.map(|icon| icons::icon(icon, 16.0, theme::ZINC_100)))
        .child(text::text(label.to_owned(), 14.0, theme::ZINC_100))
}
