use super::*;

#[test]
fn time_is_mm_ss() {
    assert_eq!(time(0.0), "00:00");
    assert_eq!(time(59.9), "00:59");
    assert_eq!(time(60.0), "01:00");
    assert_eq!(time(3599.0), "59:59");
}
