use super::*;

#[test]
fn follows_normalize_to_jota_keys() {
    assert_eq!(Follow::canonical("artist:abc").id, "artist:abc");
    assert_eq!(Follow::canonical("spotify:artist:abc").id, "artist:abc");
    assert_eq!(
        Follow::canonical("spotify:artist:abc").kind,
        FollowKind::Artist
    );
    assert_eq!(Follow::canonical("youtube:UC1").kind, FollowKind::Channel);
    assert_eq!(Follow::canonical("youtube:UC1").target(), "UC1");
    assert_eq!(Follow::canonical("bob").id, "bob");
    assert_eq!(Follow::canonical("bob").kind, FollowKind::User);
    assert_eq!(
        Follow::canonical("4uLU6hMCjMI75M1A2tKUQC").id,
        "artist:4uLU6hMCjMI75M1A2tKUQC"
    );
    assert_eq!(Follow::key(FollowKind::Channel, "UC1"), "youtube:UC1");
    assert_eq!(Follow::key(FollowKind::User, "bob"), "bob");
}
