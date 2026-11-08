use super::drag::*;

#[test]
fn drawer_curve_starts_and_ends_where_it_should() {
    assert!(bezier(0.0, 0.32, 0.72, 0.0, 1.0).abs() < 0.01);
    assert!(bezier(1.0, 0.32, 0.72, 0.0, 1.0) > 0.99);
    assert!(bezier(0.5, 0.32, 0.72, 0.0, 1.0) > 0.5);
}

#[test]
fn a_short_drag_dismisses_when_flung() {
    assert!(!dismisses(40.0, 100.0));
    assert!(dismisses(40.0, 600.0));
    assert!(dismisses(100.0, 10.0));
}
