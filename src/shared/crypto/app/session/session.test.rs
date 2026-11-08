use super::*;

#[test]
fn a_pass_round_trips_and_a_wrong_one_does_not() {
    let blob = encrypt("secret", "cookies").unwrap();
    assert!(blob.starts_with(MAGIC));
    assert_eq!(decrypt("secret", &blob).unwrap(), "cookies");
    assert!(decrypt("other", &blob).is_err());
}

#[test]
fn no_pass_leaves_the_value_untouched() {
    assert_eq!(encrypt("", "cookies").unwrap(), "cookies");
    assert!(!sealed(&encrypt("", "cookies").unwrap()));
}
