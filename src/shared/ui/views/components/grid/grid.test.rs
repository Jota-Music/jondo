use super::{BORDER, GUTTER, LEFT, cell_for, range};

fn fits(width: f32, columns: f32, gap: f32) {
    let narrow = width < 768.0;
    let column = (width - if narrow { 32.0 } else { 0.0 }).min(672.0);
    let available = column - BORDER - LEFT - GUTTER;
    let cell = cell_for(width, columns, gap);
    assert!(
        cell * columns + gap * (columns - 1.0) <= available,
        "{width}: {columns} cells of {cell} do not fit in {available}"
    );
}

#[test]
fn three_cells_fit_inside_the_panel() {
    for width in [520.0f32, 767.0, 768.0, 1120.0, 1920.0] {
        fits(width, 3.0, if width < 768.0 { 2.0 } else { 16.0 });
    }
}

#[test]
fn the_visible_range_covers_the_viewport_and_its_margin() {
    // Three columns of 100px rows in a 250px viewport start at row 0 and
    // keep two rows of margin below.
    assert_eq!(range(0.0, 250.0, 100.0, 3, 30), 0..15);
    // One row down, the first row is still inside the margin and the
    // bottom gains the row that entered the viewport.
    assert_eq!(range(100.0, 250.0, 100.0, 3, 30), 0..18);
    // Far down, the rows above the margin go.
    assert_eq!(range(1000.0, 250.0, 100.0, 3, 30), 24..30);
    // Never past the item count.
    assert_eq!(range(0.0, 250.0, 100.0, 3, 4), 0..4);
}
