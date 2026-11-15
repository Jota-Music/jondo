use super::*;

#[test]
fn an_id_carries_its_platform_and_its_kind() {
    assert_eq!(
        make(Platform::Spotify, Kind::Track, "abc"),
        "spotify:track:abc"
    );
    assert_eq!(
        parts("spotify:track:abc"),
        Some((Platform::Spotify, Kind::Track, "abc"))
    );
    assert_eq!(bare("spotify:track:abc"), "abc");
    assert_eq!(platform("youtube:track:abc"), Platform::YouTube);
}

#[test]
fn the_old_shapes_land_on_the_new_name() {
    assert_eq!(track("youtube:abc"), "youtube:track:abc");
    assert_eq!(track("local:abc"), "local:track:abc");
    assert_eq!(track("spotify:track:abc"), "spotify:track:abc");
    assert_eq!(playlist("youtube:playlist:abc"), "youtube:playlist:abc");
    assert_eq!(playlist("spotify:playlist:abc"), "spotify:playlist:abc");
    assert_eq!(playlist("local:abc"), "local:playlist:abc");
    assert_eq!(artist("artist:abc"), "spotify:artist:abc");
    assert_eq!(artist("youtube:UCabc"), "youtube:channel:UCabc");
    assert_eq!(user("user:bob"), "spotify:user:bob");
    assert_eq!(channel("UCabc"), "youtube:channel:UCabc");
}

#[test]
fn the_liked_songs_and_a_radio_keep_their_kind() {
    assert_eq!(
        playlist("spotify:user:bob:collection"),
        "spotify:collection:bob"
    );
    assert_eq!(playlist("spotify:radio:track:abc"), "spotify:radio:abc");
    assert_eq!(
        user("spotify:user:bob:collection"),
        "spotify:collection:bob"
    );
}
