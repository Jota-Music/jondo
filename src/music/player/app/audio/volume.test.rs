use super::*;

#[test]
fn the_slider_ends_map_to_silence_and_full_scale() {
    assert_eq!(gain(0.0), 0.0);
    assert_eq!(gain(1.0), 1.0);
}

#[test]
fn it_stays_linear_above_the_quiet_floor() {
    assert!((gain(0.3) - 0.2).abs() < 1e-6);
    assert!((gain(0.65) - 0.6).abs() < 1e-6);
}

#[test]
fn it_tapers_toward_silence_below_the_quiet_floor() {
    assert!((gain(0.15) - 0.05).abs() < 1e-6);
    assert!(gain(0.15) < 0.15);
}
