use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, MouseButton, ParentElement, Stateful,
    StatefulInteractiveElement, Styled, div,
};

use crate::shared::music::domain::Song;
use crate::shared::ui::views::components::text;
use crate::shared::ui::views::design::theme;
use crate::shell::app::Jondo;

use super::state::{album_part, parts};

/// Paints a song's credits as one line; artists and the album open their pages.
/// `scope` keeps the link ids unique: two rows for the same artist must not
/// share a hitbox id, or hovering one underlines them all.
#[allow(clippy::too_many_arguments)]
pub fn line(
    app: &mut Jondo,
    cx: &mut Context<Jondo>,
    scope: &str,
    song: &Song,
    album: bool,
    size: f32,
    color: gpui::Rgba,
    weight: theme::Weight,
) -> AnyElement {
    let live = !app.dragging();
    let mut row = div()
        .flex()
        .items_center()
        .min_w(theme::unit(0.0))
        .overflow_hidden()
        .whitespace_nowrap();
    for (index, part) in parts(song, album).into_iter().enumerate() {
        let Some(route) = part.route else {
            row = row.child(text::truncated(part.text, size, color, weight).flex_none());
            continue;
        };

        let id = format!("credit-{scope}-{index}");
        row = row.child(link(cx, id, part.text, size, color, weight, route, live));
    }

    row.into_any_element()
}

pub fn album(
    app: &mut Jondo,
    cx: &mut Context<Jondo>,
    scope: &str,
    song: &Song,
    size: f32,
    color: gpui::Rgba,
    weight: theme::Weight,
) -> Option<AnyElement> {
    let part = album_part(song)?;
    let live = !app.dragging();
    let row = div()
        .flex()
        .items_center()
        .min_w(theme::unit(0.0))
        .overflow_hidden()
        .whitespace_nowrap();
    Some(match part.route {
        Some(route) => row
            .child(link(
                cx,
                format!("credit-{scope}-album"),
                part.text,
                size,
                color,
                weight,
                route,
                live,
            ))
            .into_any_element(),
        None => row
            .child(text::truncated(part.text, size, color, weight).flex_none())
            .into_any_element(),
    })
}

#[allow(clippy::too_many_arguments)]
fn link(
    cx: &mut Context<Jondo>,
    id: String,
    label: String,
    size: f32,
    color: gpui::Rgba,
    weight: theme::Weight,
    route: crate::shell::app::Route,
    live: bool,
) -> Stateful<gpui::Div> {
    let element = div().id(id).flex_none();
    let element = if live {
        element
            .cursor_pointer()
            .hover(|style| style.underline())
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_click(cx.listener(move |app, _: &gpui::ClickEvent, _, cx| {
                cx.stop_propagation();
                app.open(route.clone(), cx);
                cx.notify();
            }))
    } else {
        element
    };

    element.child(text::truncated(label, size, color, weight))
}
