use gpui::{BorderStyle, Corners, Div, Edges, ParentElement, Rgba, Styled, canvas, div, px, quad};

use crate::shared::ui::views::design::theme;

/// A box whose fill and 1px border are painted as a single quad.
///
/// `Style::paint` paints a background and a border as two quads over the same
/// area, so every bordered surface costs its whole area twice. On a phone the
/// second pass is not free: the fragment shader runs over the full rect even
/// when only the border is visible. Drawing both in one quad removes that pass
/// and paints the same pixels, because a child never covers the border here.
pub fn surface(radius: f32, fill: Rgba, border: Rgba) -> Div {
    let corner = theme::unit(radius);
    div().relative().rounded(corner).child(
        canvas(
            |_, _, _| (),
            move |bounds, (), window, _| {
                window.paint_quad(quad(
                    bounds,
                    Corners::all(corner),
                    fill,
                    Edges::all(px(1.0)),
                    border,
                    BorderStyle::Solid,
                ));
            },
        )
        .absolute()
        .inset_0(),
    )
}
