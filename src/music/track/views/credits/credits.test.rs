use super::state::{album_part, parts};
use crate::shared::music::domain::{Song, Source};
use crate::shell::app::Route;

fn song() -> Song {
    Song {
        artists: vec!["One".to_owned(), "Two".to_owned()],
        artist_ids: vec!["a".to_owned(), String::new()],
        album: "Record".to_owned(),
        album_id: Some("b".to_owned()),
        source: Source::Spotify,
        ..Default::default()
    }
}

#[test]
fn artists_and_album_link_when_the_ids_exist() {
    let parts = parts(&song(), true);
    assert_eq!(parts.len(), 5);
    assert_eq!(parts[0].route, Some(Route::Artist("a".to_owned())));
    assert!(parts[1].route.is_none(), "separator stays plain");
    assert!(parts[2].route.is_none(), "an artist without an id is text");
    assert!(parts[3].route.is_none(), "separator stays plain");
    assert_eq!(parts[4].route, Some(Route::Album("b".to_owned())));
}

#[test]
fn the_album_can_be_left_out() {
    let parts = parts(&song(), false);
    assert_eq!(parts.len(), 3);
    assert!(parts.iter().all(|part| part.text != "Record"));
}

#[test]
fn the_album_can_be_painted_alone() {
    let part = album_part(&song()).expect("an album with a name is painted");
    assert_eq!(part.text, "Record");
    assert_eq!(part.route, Some(Route::Album("b".to_owned())));
    assert!(album_part(&Song::default()).is_none(), "no album, no line");
}

#[test]
fn youtube_playlists_open_the_playlist() {
    let mut song = song();
    song.source = Source::YouTube;
    song.playlist_id = Some("PLabc".to_owned());
    assert_eq!(
        parts(&song, true).last().unwrap().route,
        Some(Route::Playlist("youtube:playlist:PLabc".to_owned()))
    );
}

#[test]
fn youtube_artists_open_their_channel() {
    let mut song = song();
    song.source = Source::YouTube;
    assert_eq!(
        parts(&song, false)[0].route,
        Some(Route::Channel("a".to_owned()))
    );
}
