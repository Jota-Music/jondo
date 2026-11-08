use super::progress;

#[test]
fn closing_runs_the_same_curve_backwards() {
    assert_eq!(progress(0.0, false), 0.0);
    assert_eq!(progress(0.25, false), 0.25);
    assert_eq!(progress(1.0, false), 1.0);
    assert_eq!(progress(0.0, true), 1.0);
    assert_eq!(progress(0.25, true), 0.75);
    assert_eq!(progress(1.0, true), 0.0);
}
