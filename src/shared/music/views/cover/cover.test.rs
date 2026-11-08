use super::rounded_inset;

#[test]
fn a_round_tile_insets_its_band_to_the_circle() {
    assert_eq!(rounded_inset(64.0, 64.0, 32.0, 0.0), 32.0);
    assert!((rounded_inset(64.0, 64.0, 32.0, 32.0) - 0.0).abs() < 0.001);
    assert_eq!(rounded_inset(64.0, 64.0, 32.0, 64.0), 32.0);
    assert!(rounded_inset(64.0, 64.0, 32.0, 8.0) > 0.0);
}

#[test]
fn a_square_tile_keeps_the_band_full_height() {
    assert_eq!(rounded_inset(160.0, 160.0, 0.0, 0.0), 0.0);
    assert_eq!(rounded_inset(160.0, 160.0, 6.0, 80.0), 0.0);
}
