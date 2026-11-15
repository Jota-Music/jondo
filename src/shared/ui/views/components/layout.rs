use std::cell::Cell;

use gpui::{Div, IntoElement, ParentElement, Styled, div};

use crate::shared::ui::views::design::theme;

use super::surface;

// Thread-local so a test's frames never see another test's window state: the
// shell publishes these on every paint, and layout reads them in the same
// thread. `theme::SCALE` already keeps the same assumption.
thread_local! {
    /// The phone breakpoint, written by the shell on every paint. A panel or a
    /// head deep in a page bleeds its frame there, and this keeps the window
    /// out of the signature of every view that only needs to know which side
    /// of it it is on.
    static NARROW: Cell<bool> = const { Cell::new(false) };

    /// Whether the ambient glow is behind the page. The glow lifts every
    /// surface it sits under, so the boxes that sit on it directly (a cover
    /// slot, a scrollbar thumb) pick a tone for it.
    static AMBIENT: Cell<bool> = const { Cell::new(false) };
}

pub fn set_narrow(narrow: bool) {
    NARROW.with(|value| value.set(narrow));
}

pub fn narrow() -> bool {
    NARROW.with(Cell::get)
}

pub fn set_ambient(on: bool) {
    AMBIENT.with(|value| value.set(on));
}

pub fn ambient() -> bool {
    AMBIENT.with(Cell::get)
}

/// The 672px column the application centers on every screen.
pub fn page(inner: impl IntoElement) -> Div {
    div().size_full().flex().justify_center().child(
        div()
            .w(theme::unit(theme::CONTENT_WIDTH))
            .h_full()
            .max_w_full()
            .min_w(theme::unit(0.0))
            .child(inner),
    )
}

/// The same 672px column without forcing the height, for panels inside a column.
pub fn column(inner: impl IntoElement) -> Div {
    div().w_full().flex().justify_center().child(
        div()
            .w(theme::unit(theme::CONTENT_WIDTH))
            .max_w_full()
            .min_w(theme::unit(0.0))
            .child(inner),
    )
}

/// The surface a page or panel paints its content on. No fill on purpose: the
/// ambient glow reaches every list, and only the border says where the panel
/// ends. On a phone the box gives way to the page and keeps a top hairline.
pub fn panel(inner: impl IntoElement) -> Div {
    if narrow() {
        return div()
            .flex_1()
            .min_h(theme::unit(0.0))
            .w_full()
            .overflow_hidden()
            .border_t_1()
            .border_color(theme::BORDER)
            .child(inner);
    }

    surface(8.0, gpui::transparent_black().into(), theme::BORDER)
        .flex_1()
        .min_h(theme::unit(0.0))
        .w_full()
        .overflow_hidden()
        .child(inner)
}
