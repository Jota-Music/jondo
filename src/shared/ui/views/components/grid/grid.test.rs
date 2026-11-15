use super::{BORDER, GUTTER, LEFT, cell_for, gutter_for, left_for};

fn fits(width: f32, columns: f32, gap: f32) {
    let narrow = width < 768.0;
    let column = width.min(672.0);
    let border = if narrow { 0.0 } else { BORDER };
    let available = column - border - left_for(width) - gutter_for(width);
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

/// A phone keeps Jota's `pl-2 pr-4` and its 2px grid gap, so the cards are the
/// same square the web build draws.
#[test]
fn a_phone_cell_matches_jotas_shelf() {
    for width in [360.0f32, 390.0, 412.0, 600.0, 672.0] {
        let jota = ((width - 8.0 - 16.0 - 2.0 * 2.0) / 3.0).floor();
        assert_eq!(cell_for(width, 3.0, 2.0), jota, "{width}");
    }
}

#[test]
fn the_desktop_insets_stay_as_they_were() {
    assert_eq!(left_for(800.0), LEFT);
    assert_eq!(gutter_for(800.0), GUTTER);
}
