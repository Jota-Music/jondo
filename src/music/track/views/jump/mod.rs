use gpui::{
    Div, InteractiveElement, ParentElement, Pixels, Stateful, StatefulInteractiveElement, Styled,
    UniformListScrollHandle, div,
};

use crate::shared::i18n::t;
use crate::shared::music::domain::Song;
use crate::shared::music::views::cover;
use crate::shared::ui::views::components::text;
use crate::shared::ui::views::design::icons;
use crate::shared::ui::views::design::theme;
use crate::shell::app::Jondo;
use gpui_kit::component::tooltip::Tooltip;

/// The pill sits this far from the bottom of the list and keeps this much
/// height. The list reserves that plus a little air, so the last row never
/// ends up flush against the pill.
pub const BOTTOM: f32 = 12.0;
pub const HEIGHT: f32 = 32.0;
pub const AIR: f32 = 8.0;
pub const GAP: f32 = BOTTOM + HEIGHT + AIR;

/// Where a row sits relative to the visible slice of a uniform list: `Some(true)`
/// is above it, `Some(false)` below it, and `None` while it is in view or before
/// the list has laid out. `top` is the list's top padding.
///
/// A uniform list scrolls by the height it measured, which is not always the
/// height the row asked for: the border and the layout rounding make the two
/// drift, and over a long playlist the drift is what put the pill on screen
/// for a row that was already visible. The measured height is the one the list
/// itself uses, `contents / count`.
pub fn offscreen(
    scroll: &UniformListScrollHandle,
    row: usize,
    count: usize,
    top: f32,
) -> Option<bool> {
    let state = scroll.0.borrow();
    let size = state.last_item_size?;
    place(
        state.base_handle.offset().y,
        size.item.height,
        size.contents.height,
        top,
        row,
        count,
    )
}

fn place(
    offset: Pixels,
    viewport: Pixels,
    content: Pixels,
    top: f32,
    row: usize,
    count: usize,
) -> Option<bool> {
    if count == 0 {
        return None;
    }

    let height = content.as_f32() / count as f32;
    if height <= 0.0 {
        return None;
    }

    let max = (content - viewport).as_f32().max(0.0);
    let scrolled = (-offset.as_f32()).clamp(0.0, max);
    let first = ((scrolled - top).max(0.0) / height).floor() as usize;
    let rows = (viewport.as_f32() / height).ceil().max(1.0) as usize;
    if row < first {
        Some(true)
    } else if row >= first + rows {
        Some(false)
    } else {
        None
    }
}

/// Whether the list has reached its end, without counting the room the pill
/// takes: the gap belongs to the last row alone, so it appears only there.
/// The element bounds keep the answer independent of that room, which would
/// otherwise move the end out from under the offset.
pub fn ending(scroll: &UniformListScrollHandle) -> bool {
    let state = scroll.0.borrow();
    let Some(size) = state.last_item_size else {
        return false;
    };

    let viewport = state.base_handle.bounds().size.height;
    let max = (size.contents.height - viewport).as_f32().max(0.0);
    let scrolled = (-state.base_handle.offset().y).as_f32().max(0.0);
    max - scrolled <= 0.5
}

/// The pill a list carries while the playing row is outside the viewport: its
/// cover, its name and the direction it lies in, so one tap scrolls back.
pub fn pill(
    app: &mut Jondo,
    id: impl Into<gpui::ElementId>,
    song: &Song,
    above: bool,
) -> Stateful<Div> {
    let art = cover::art(
        &mut app.covers,
        song.cover.as_deref().unwrap_or_default(),
        20.0,
        20.0,
        10.0,
    );
    div()
        .id(id)
        .absolute()
        .bottom(theme::unit(BOTTOM))
        .left(gpui::relative(0.5))
        .ml(theme::unit(-110.0))
        .w(theme::unit(220.0))
        .h(theme::unit(HEIGHT))
        .flex()
        .items_center()
        .gap(theme::unit(8.0))
        .pl(theme::unit(6.0))
        .pr(theme::unit(10.0))
        .rounded_full()
        .bg(theme::alpha(theme::ZINC_900, 0.92))
        .border_1()
        .border_color(theme::BORDER)
        .shadow_lg()
        .cursor(gpui::CursorStyle::PointingHand)
        .hover(|this| this.bg(theme::ZINC_800))
        .tooltip(|window, cx| Tooltip::new(t("Jump to current song")).build(window, cx))
        .child(art)
        .child(
            text::truncated(
                song.name.clone(),
                12.0,
                theme::ZINC_300,
                theme::Weight::Regular,
            )
            .flex_1(),
        )
        .child(icons::icon(
            if above {
                icons::CHEVRON_UP
            } else {
                icons::CHEVRON_DOWN
            },
            14.0,
            theme::MUTED,
        ))
}

#[cfg(test)]
#[path = "jump.test.rs"]
mod tests;
