use super::*;
use crate::shared::music::domain as music;

fn spotify_song() -> music::Song {
    music::Song {
        id: "spotify:track:abc".to_owned(),
        name: "Song".to_owned(),
        artists: vec!["One".to_owned(), "Two".to_owned()],
        artist_ids: vec![
            "spotify:artist:one".to_owned(),
            "spotify:artist:two".to_owned(),
        ],
        album: "Album".to_owned(),
        album_id: Some("spotify:album:one".to_owned()),
        album_year: 1999,
        playlist_id: Some("spotify:playlist:one".to_owned()),
        cover: Some("https://i.scdn.co/image/cover".to_owned()),
        duration: 214,
        source: music::Source::Spotify,
        video_id: None,
        uri: Some("spotify:track:abc".to_owned()),
        broken: false,
    }
}

/// A song keeps what the Jota file holds, and comes back with what the app
/// shows: names, ids, album, cover and duration.
#[test]
fn a_song_round_trips_through_jotas_shape() {
    let song = spotify_song();
    let stored = Song::from(&song);
    let back: music::Song = stored.clone().into();

    assert_eq!(stored.share.id, "abc");
    assert_eq!(stored.share.url, "https://open.spotify.com/track/abc");
    assert_eq!(stored.album.url, "spotify:album:one");
    assert_eq!(stored.album.name, "Album");

    assert_eq!(back.id, song.id);
    assert_eq!(back.name, song.name);
    assert_eq!(back.artists, song.artists);
    assert_eq!(back.artist_ids, song.artist_ids);
    assert_eq!(back.album, song.album);
    assert_eq!(back.album_id, song.album_id);
    assert_eq!(back.cover, song.cover);
    assert_eq!(back.duration, song.duration);
    assert_eq!(back.uri, song.uri);
    assert_eq!(back.source, song.source);
}

/// A YouTube song writes its video alone, the way Jota names it.
#[test]
fn a_youtube_song_writes_the_video_id_bare() {
    let mut song = spotify_song();
    song.id = "youtube:track:video".to_owned();
    song.source = music::Source::YouTube;
    song.video_id = Some("youtube:track:video".to_owned());
    song.uri = None;

    let stored = Song::from(&song);
    assert_eq!(stored.video_id, "video");
    assert_eq!(stored.share, Share::default());
    assert_eq!(stored.url, "");

    let back: music::Song = stored.into();
    assert_eq!(back.video_id.as_deref(), Some("youtube:track:video"));
    assert_eq!(back.source, music::Source::YouTube);
}

/// The playlist holds Jota's songs, with no id of its own.
#[test]
fn a_playlist_loses_only_what_jota_does_not_cache() {
    let playlist = music::Playlist {
        id: "spotify:playlist:one".to_owned(),
        name: "Mix".to_owned(),
        cover: Some("cover".to_owned()),
        owner: Some("Owner".to_owned()),
        songs: vec![spotify_song()],
        ..Default::default()
    };

    let stored = Playlist::from(&playlist);
    let back = stored.into_domain(&playlist.id);

    assert_eq!(back.id, playlist.id);
    assert_eq!(back.name, playlist.name);
    assert_eq!(back.songs.len(), 1);
    // what Jota does not cache comes back from the songs
    assert_eq!(
        back.covers,
        vec!["https://i.scdn.co/image/cover".to_owned()]
    );
    assert_eq!(back.track_count, 1);
}

/// The header counts what the source says the playlist holds, which is more
/// than the songs that resolve: an unavailable track still takes a slot.
#[test]
fn a_cached_playlist_keeps_the_count_its_source_reported() {
    let playlist = music::Playlist {
        id: "spotify:playlist:one".to_owned(),
        name: "Mix".to_owned(),
        track_count: 12,
        songs: vec![spotify_song()],
        ..Default::default()
    };

    let back = Playlist::from(&playlist).into_domain(&playlist.id);
    assert_eq!(back.track_count, 12);

    // a file written before the count was kept falls back to the songs
    let old = Playlist {
        track_count: 0,
        ..Playlist::from(&playlist)
    };
    assert_eq!(old.into_domain(&playlist.id).track_count, 1);
}
