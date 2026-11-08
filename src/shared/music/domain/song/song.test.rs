use super::*;

#[test]
fn query_joins_the_name_and_the_first_artist() {
    let song = Song {
        name: "Song".to_owned(),
        artists: vec!["Artist".to_owned(), "Other".to_owned()],
        ..Default::default()
    };

    assert_eq!(song.query(), "Song Artist");
    assert_eq!(Song::default().query(), "");
}

#[test]
fn youtube_song_keeps_the_video_id() {
    let song = youtube_song("abc12345678", "Title", "Artist", 120);
    assert_eq!(song.video(), Some("abc12345678"));
    assert_eq!(song.id, "youtube:abc12345678");
}
