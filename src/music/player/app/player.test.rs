use super::*;

#[test]
fn settle_keeps_the_loader_until_the_engine_plays() {
    let mut player = Player::new(1.0, false);
    assert!(!player.settle());

    player.loading = true;
    assert!(!player.settle());
    assert!(player.loading);
}

#[test]
fn stop_clears_the_loader() {
    let mut player = Player::new(1.0, false);
    player.loading = true;
    player.stop();
    assert!(!player.loading);
}
