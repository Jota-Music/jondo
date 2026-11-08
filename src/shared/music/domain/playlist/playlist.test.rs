use super::*;

#[test]
fn liked_songs_is_a_user_context() {
    let liked = Playlist {
        id: "spotify:user:bob:collection".to_owned(),
        ..Default::default()
    };

    assert!(liked.liked());
    assert!(
        !Playlist {
            id: "spotify:playlist:abc".to_owned(),
            ..Default::default()
        }
        .liked()
    );
}
