use crate::shared::ui::views::design::theme;

use super::*;

fn slice(rows: usize, height: f32) -> Pixels {
    theme::unit(height * rows as f32)
}

#[test]
fn the_row_outside_the_slice_names_its_direction() {
    let viewport = theme::unit(400.0);
    let content = slice(10, 64.0);
    assert_eq!(place(theme::unit(0.0), viewport, content, 0.0, 0, 10), None);
    assert_eq!(
        place(theme::unit(-100.0), viewport, content, 0.0, 0, 10),
        Some(true)
    );
    assert_eq!(
        place(theme::unit(-100.0), viewport, content, 0.0, 4, 10),
        None
    );
    assert_eq!(
        place(theme::unit(-100.0), viewport, content, 0.0, 8, 10),
        Some(false)
    );
    assert_eq!(place(theme::unit(0.0), viewport, content, 0.0, 0, 0), None);
}

#[test]
fn the_measured_height_is_the_one_the_list_scrolls_by() {
    // The row asks for 64px but the list measures a little more: at the
    // bottom the offset of the nominal height put the last row below the
    // slice and the pill claimed it was above, over a row on screen.
    let count = 3742;
    let height = 64.285_71;
    let content = slice(count, height);
    let viewport = theme::unit(424.285_74);
    let offset = -(content - viewport);
    assert_eq!(
        place(offset, viewport, content, 0.0, count - 1, count),
        None
    );
    assert_eq!(place(offset, viewport, content, 0.0, 5, count), Some(true));
}
