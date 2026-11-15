use std::fs;

use super::{get, name, put, save_library};
use crate::shared::music::domain::{Playlist, Song, Source};
use crate::shared::storage::cache;

fn song(id: &str) -> Song {
    Song {
        id: id.to_owned(),
        name: "Song".to_owned(),
        artists: vec!["Artist".to_owned()],
        artist_ids: vec!["spotify:artist:one".to_owned()],
        album: "Album".to_owned(),
        album_id: Some("spotify:album:one".to_owned()),
        cover: Some("https://i.scdn.co/image/cover".to_owned()),
        duration: 214,
        source: Source::Spotify,
        uri: Some(id.to_owned()),
        ..Default::default()
    }
}

fn stored(name: &str) -> serde_json::Value {
    let text = fs::read_to_string(cache::path(name).unwrap()).unwrap();
    serde_json::from_str(&text).unwrap()
}

/// Liked Songs is a user context: a bare playlist under `contexts/`, with the
/// URI the source named. A radio is never written at all.
#[test]
fn a_context_and_a_radio_follow_jotas_rule() {
    let id = "spotify:collection:alice";
    put(&Playlist {
        id: id.to_owned(),
        songs: vec![song("spotify:track:context")],
        ..Default::default()
    });

    let stored = stored("spotify/contexts/spotify:user:alice:collection");
    assert!(stored["value"].get("revision").is_none(), "{stored}");
    assert!(stored["value"]["songs"].is_array(), "{stored}");
    assert_eq!(get(id).expect("cached context").0.id, id);

    assert!(name("spotify:radio:track:abc").is_none());
}

#[test]
fn playlists_get_their_own_file() {
    let spotify = cache::path(&name("spotify:playlist:abc").unwrap()).unwrap();
    let youtube = cache::path(&name("youtube:playlist:abc").unwrap()).unwrap();
    assert_ne!(spotify, youtube);
    let file = spotify.file_name().unwrap().to_str().unwrap();
    assert!(file.starts_with("spotify_playlist_abc-"), "{file}");
    assert!(file.ends_with(".json"));
    assert_eq!(
        cache::path(&name("spotify:playlist:abc").unwrap()).unwrap(),
        spotify
    );
}

#[test]
fn playlist_survives_a_roundtrip() {
    let playlist = Playlist {
        id: "spotify:playlist:roundtrip".to_owned(),
        name: "Mix".to_owned(),
        cover: Some("https://i.scdn.co/image/cover".to_owned()),
        songs: vec![song("spotify:track:one")],
        ..Default::default()
    };

    put(&playlist);

    let (cached, fresh) = get("spotify:playlist:roundtrip").expect("cached playlist");
    assert!(fresh);
    assert_eq!(cached.id, playlist.id);
    assert_eq!(cached.name, "Mix");
    assert_eq!(cached.songs.len(), 1);
    assert_eq!(cached.songs[0].artists, ["Artist"]);
    assert_eq!(cached.songs[0].album, "Album");
    assert_eq!(
        cached.cover.as_deref(),
        Some("https://i.scdn.co/image/cover")
    );
    assert!(!cached.loading);
}

/// The file is Jota's: the revision it was fetched at around the playlist,
/// whose id is the cache key and not a field.
#[test]
fn the_playlist_file_is_jotas() {
    let id = "spotify:playlist:file";
    put(&Playlist {
        id: id.to_owned(),
        name: "Mix".to_owned(),
        songs: vec![song("spotify:track:file")],
        ..Default::default()
    });

    let stored = stored(&name(id).unwrap());
    assert!(stored["value"].get("revision").is_some(), "{stored}");
    assert!(stored["value"]["playlist"]["songs"].is_array(), "{stored}");
    assert!(
        stored["value"]["playlist"].get("id").is_none(),
        "the id is the cache key, not a field"
    );
    assert_eq!(
        stored["value"]["playlist"]["songs"][0]["album"]["title"],
        "Album"
    );
    assert!(
        stored["value"]["playlist"]["songs"][0]
            .get("youtubeId")
            .is_none()
    );
}

/// The library is a list of summaries under the account, as Jota writes it.
#[test]
fn the_library_is_a_summary_list_under_the_user() {
    save_library(
        "alice",
        &[Playlist {
            id: "spotify:playlist:library".to_owned(),
            name: "Mix".to_owned(),
            ..Default::default()
        }],
    );

    let stored = stored("spotify/library/alice");
    assert!(stored["value"].is_array(), "{stored}");
    assert_eq!(stored["value"][0]["id"], "spotify:playlist:library");
    assert!(stored["value"][0].get("songs").is_none(), "{stored}");
}
