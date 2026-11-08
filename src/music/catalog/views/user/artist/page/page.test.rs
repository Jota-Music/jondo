use super::tab_widths;

const GAP: f32 = 8.0;

#[test]
fn below_640_the_tabs_keep_jotas_two_columns() {
    for width in [360.0, 480.0, 639.0] {
        let (column, pair, tail, stacked) = tab_widths(width);
        assert!(stacked, "{width} should stack");
        assert!((pair - tail).abs() < 0.01, "{width} should halve");
        assert!((pair * 2.0 + GAP - column).abs() < 0.01);
    }
}

#[test]
fn from_640_the_tabs_use_jotas_six_columns_without_wrapping() {
    for width in [640.0, 700.0, 767.0, 768.0, 1024.0, 1920.0] {
        let (column, pair, tail, stacked) = tab_widths(width);
        assert!(!stacked, "{width} should not stack");
        assert!(pair * 2.0 + GAP <= column + 0.01, "{width}");
        assert!(tail * 3.0 + GAP * 2.0 <= column + 0.01, "{width}");
    }
}
