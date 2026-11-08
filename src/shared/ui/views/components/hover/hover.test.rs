use super::*;

#[test]
fn a_forgotten_surface_stops_fading() {
    let id = ElementId::Name("hover-test".into());
    let then = Instant::now() - DURATION;
    set(&id, true, then);
    assert!(value(&id, Instant::now()) > 0.9);
    forget(&id);
    assert_eq!(value(&id, Instant::now()), 0.0);
    assert!(!running());
}
