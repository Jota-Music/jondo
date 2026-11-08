use super::{get, path, put};
use crate::shared::music::domain::{Playlist, Song};

#[test]
fn playlists_get_their_own_file() {
    let spotify = path("spotify:playlist:abc").unwrap();
    let youtube = path("youtube:playlist:abc").unwrap();
    assert_ne!(spotify, youtube);
    assert_eq!(
        spotify.file_name().unwrap().to_str().unwrap(),
        "spotify_playlist_abc.json"
    );
    assert_eq!(path("spotify:playlist:abc").unwrap(), spotify);
}

#[test]
#[ignore = "writes to the config dir; point XDG_CONFIG_HOME at a scratch directory"]
fn playlist_survives_a_roundtrip() {
    let playlist = Playlist {
        id: "spotify:playlist:roundtrip".to_owned(),
        songs: vec![Song {
            id: "song".to_owned(),
            name: "Song".to_owned(),
            ..Default::default()
        }],
        ..Default::default()
    };

    put(&playlist);

    let (cached, fresh) = get("spotify:playlist:roundtrip").expect("cached playlist");
    assert!(fresh);
    assert_eq!(cached.id, playlist.id);
    assert_eq!(cached.songs.len(), 1);
    assert!(!cached.loading);
}
