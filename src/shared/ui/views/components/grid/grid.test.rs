use super::{BORDER, GUTTER, LEFT, cell_for};

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
