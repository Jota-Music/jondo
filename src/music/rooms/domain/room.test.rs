use super::RoomStatus;

// A room tile paints the relay's cached playback, so the status payload has
// to keep the song Jota put in it.
#[test]
fn room_status_keeps_the_cached_song() {
    let json = r##"{
        "active": true, "members": 2, "hasHost": true, "locked": false,
        "state": {
            "at": 1794085943814, "binary": "#000000", "color": "#ff0000",
            "index": 0, "playing": true, "positionMs": 12345,
            "repeat": "off", "shuffle": false, "songId": "youtube:abc123",
            "song": {
                "album": {
                    "covers": ["https://cover"], "id": "album1",
                    "title": "Album Name", "url": ""
                },
                "artists": [{ "id": "", "name": "Artist Name", "source": "" }],
                "broken": false, "duration": 200, "id": "youtube:abc123",
                "name": "Track Name", "share": { "id": "abc123", "url": "" },
                "url": "", "youtubeId": "abc123"
            },
            "youtubeId": "abc123", "t": "state"
        }
    }"##;
    let status: RoomStatus = serde_json::from_str(json).expect("relay status");
    let song = status
        .state
        .expect("cached playback")
        .song
        .expect("cached song");
    assert_eq!(song.name, "Track Name");
    assert_eq!(song.artists[0].name, "Artist Name");
    assert_eq!(song.album.covers, ["https://cover"]);
    assert_eq!(song.youtube_id.as_deref(), Some("abc123"));
}
