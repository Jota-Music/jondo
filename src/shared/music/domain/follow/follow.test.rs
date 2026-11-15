use super::*;

#[test]
fn follows_normalize_to_the_names_the_app_writes() {
    assert_eq!(Follow::canonical("artist:abc").id, "spotify:artist:abc");
    assert_eq!(
        Follow::canonical("spotify:artist:abc").id,
        "spotify:artist:abc"
    );
    assert_eq!(
        Follow::canonical("spotify:artist:abc").kind,
        FollowKind::Artist
    );
    assert_eq!(Follow::canonical("youtube:UC1").kind, FollowKind::Channel);
    assert_eq!(Follow::canonical("youtube:UC1").id, "youtube:channel:UC1");
    assert_eq!(Follow::canonical("youtube:UC1").target(), "UC1");
    assert_eq!(Follow::canonical("bob").id, "spotify:user:bob");
    assert_eq!(Follow::canonical("bob").kind, FollowKind::User);
    assert_eq!(
        Follow::canonical("4uLU6hMCjMI75M1A2tKUQC").id,
        "spotify:artist:4uLU6hMCjMI75M1A2tKUQC"
    );
    assert_eq!(
        Follow::key(FollowKind::Channel, "UC1"),
        "youtube:channel:UC1"
    );
    assert_eq!(Follow::key(FollowKind::User, "bob"), "spotify:user:bob");
}
