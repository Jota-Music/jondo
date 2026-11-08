#[cfg(test)]
mod table_tests {
    use crate::shared::music::domain::Song;
    use crate::shell::app::{Order, Table};

    fn song(name: &str, duration: u32) -> Song {
        Song {
            id: name.to_lowercase(),
            name: name.to_owned(),
            duration,
            ..Default::default()
        }
    }

    #[test]
    fn a_table_points_its_rows_at_the_songs_and_ids_it_carries() {
        let table = Table::build(
            vec![song("Beta", 200), song("Alpha", 100)],
            "alpha",
            Order::Added,
            true,
        );

        assert_eq!(*table.rows, vec![1]);
        assert_eq!(*table.ids, vec!["alpha".to_owned()]);
        assert_eq!(table.full.len(), 2);
    }

    #[test]
    fn a_plain_table_keeps_every_song_in_source_order() {
        let table = Table::build(
            vec![song("Beta", 200), song("Alpha", 100)],
            "",
            Order::DurationAsc,
            false,
        );

        assert_eq!(*table.rows, vec![0, 1]);
        assert_eq!(*table.ids, vec!["beta".to_owned(), "alpha".to_owned()]);
    }
}

#[cfg(test)]
mod link_tests {
    use crate::shared::music::domain::link::{
        YoutubeRef, direct_url, spotify_id, spotify_link, youtube_ref,
    };

    use crate::shell::app::Route;
    use crate::shell::app::clipboard::{Clipboard, Kind, Link, Target, parse};

    #[test]
    fn spotify_links_resolve_to_their_page() {
        assert_eq!(
            spotify_link("spotify:album:4aawyAB9vmqN3uQ7FjRGTy"),
            Some(("album", "4aawyAB9vmqN3uQ7FjRGTy".to_owned()))
        );
        assert_eq!(
            spotify_link("https://open.spotify.com/artist/1dfeR4HaWDbWqFHLkxsg1d?si=x"),
            Some(("artist", "1dfeR4HaWDbWqFHLkxsg1d".to_owned()))
        );
        assert_eq!(
            spotify_link("https://open.spotify.com/intl-es/playlist/37i9dQZF1DXcBWIGoYBM5M"),
            Some(("playlist", "37i9dQZF1DXcBWIGoYBM5M".to_owned()))
        );
        assert_eq!(
            spotify_link("https://play.spotify.com/track/4uLU6hMCjMI75M1A2tKUQC"),
            Some(("track", "4uLU6hMCjMI75M1A2tKUQC".to_owned()))
        );
        assert_eq!(
            spotify_link("open.spotify.com/embed/album/4aawyAB9vmqN3uQ7FjRGTy"),
            Some(("album", "4aawyAB9vmqN3uQ7FjRGTy".to_owned()))
        );
        assert_eq!(
            spotify_link("https://open.spotify.com/track/4uLU6hMCjMI75M1A2tKUQC"),
            Some(("track", "4uLU6hMCjMI75M1A2tKUQC".to_owned()))
        );
        assert_eq!(
            spotify_link("spotify:user:alice"),
            Some(("user", "alice".to_owned()))
        );
        assert_eq!(spotify_link("queen"), None);
        assert_eq!(
            spotify_link("https://open.spotify.com/album/not-an-id"),
            None
        );
    }

    #[test]
    fn a_bare_reference_is_an_id_only_when_it_has_id_shape() {
        assert!(spotify_id("4aawyAB9vmqN3uQ7FjRGTy"));
        assert!(!spotify_id("queen"));
        assert!(!spotify_id("4aawyAB9vmqN3uQ7FjRGTy!"));
    }

    #[test]
    fn youtube_links_name_the_page_they_open() {
        assert_eq!(
            youtube_ref("https://www.youtube.com/playlist?list=PL1234567890abcdef"),
            Some(YoutubeRef::Playlist("PL1234567890abcdef".to_owned()))
        );
        assert_eq!(
            youtube_ref("https://music.youtube.com/watch?v=dQw4w9WgXcQ&list=OLAK5uy_kABCDEFGHIJ"),
            Some(YoutubeRef::Video("dQw4w9WgXcQ".to_owned()))
        );
        assert_eq!(
            youtube_ref("PL1234567890abcdef"),
            Some(YoutubeRef::Playlist("PL1234567890abcdef".to_owned()))
        );
        assert_eq!(
            youtube_ref("dQw4w9WgXcQ"),
            Some(YoutubeRef::Video("dQw4w9WgXcQ".to_owned()))
        );
        assert_eq!(
            youtube_ref("https://youtu.be/dQw4w9WgXcQ"),
            Some(YoutubeRef::Video("dQw4w9WgXcQ".to_owned()))
        );
        assert_eq!(
            youtube_ref("https://www.youtube.com/channel/UCq19-LqvG35A-30oyAiPiqA"),
            Some(YoutubeRef::Channel("UCq19-LqvG35A-30oyAiPiqA".to_owned()))
        );
        assert_eq!(
            youtube_ref("https://www.youtube.com/@jota"),
            Some(YoutubeRef::Channel("@jota".to_owned()))
        );
        assert_eq!(youtube_ref("lofi beats"), None);
        assert_eq!(
            youtube_ref("https://open.spotify.com/playlist/37i9dQZF1DXcBWIGoYBM5M"),
            None
        );
    }

    #[test]
    fn only_an_absolute_web_address_is_a_direct_query() {
        assert!(direct_url("https://example.com/watch"));
        assert!(!direct_url("example.com/watch"));
        assert!(!direct_url("lofi beats"));
    }

    #[test]
    fn a_copied_link_opens_the_page_it_names() {
        assert_eq!(
            parse("https://open.spotify.com/playlist/37i9dQZF1DXcBWIGoYBM5M"),
            Some(Link {
                kind: Kind::Playlist,
                text: "https://open.spotify.com/playlist/37i9dQZF1DXcBWIGoYBM5M".to_owned(),
                target: Target::Route(Route::Playlist(
                    "spotify:playlist:37i9dQZF1DXcBWIGoYBM5M".to_owned()
                )),
            })
        );
        assert_eq!(
            parse("spotify:track:4cOdK2wGLETKBW3PvgPWqT"),
            Some(Link {
                kind: Kind::Song,
                text: "spotify:track:4cOdK2wGLETKBW3PvgPWqT".to_owned(),
                target: Target::Song("spotify:track:4cOdK2wGLETKBW3PvgPWqT".to_owned()),
            })
        );
        assert_eq!(
            parse("https://youtu.be/dQw4w9WgXcQ"),
            Some(Link {
                kind: Kind::Song,
                text: "https://youtu.be/dQw4w9WgXcQ".to_owned(),
                target: Target::Song("youtube:dQw4w9WgXcQ".to_owned()),
            })
        );
    }

    #[test]
    fn a_playlist_parameter_wins_on_the_clipboard() {
        assert_eq!(
            parse("https://www.youtube.com/watch?v=dQw4w9WgXcQ&list=PL1234567890abcdef&index=3"),
            Some(Link {
                kind: Kind::Playlist,
                text: "https://www.youtube.com/watch?v=dQw4w9WgXcQ&list=PL1234567890abcdef&index=3"
                    .to_owned(),
                target: Target::Route(Route::Playlist(
                    "youtube:playlist:PL1234567890abcdef".to_owned()
                )),
            })
        );
    }

    #[test]
    fn anything_that_is_not_a_known_link_is_ignored() {
        assert_eq!(parse("just some text"), None);
        assert_eq!(parse("local:4cOdK2wGLETKBW3PvgPWqT"), None);
        assert_eq!(
            parse("https://example.com/playlist/37i9dQZF1DXcBWIGoYBM5M"),
            None
        );
        assert_eq!(parse(""), None);
    }

    #[test]
    fn the_same_copied_text_is_offered_once() {
        let mut clipboard = Clipboard::default();
        assert!(clipboard.detect("https://youtu.be/dQw4w9WgXcQ"));
        clipboard.dismiss();
        assert!(!clipboard.detect("https://youtu.be/dQw4w9WgXcQ"));
        assert!(clipboard.detect("https://youtu.be/aaaaaaaaaaa"));
    }
}

#[cfg(test)]
mod source_tests {
    use crate::shared::music::domain::Source;
    use crate::shell::app::setup::chosen_source;

    #[test]
    fn an_account_keeps_the_chosen_source_while_it_connects() {
        assert_eq!(chosen_source(true, Source::Spotify), Source::Spotify);
        assert_eq!(chosen_source(true, Source::YouTube), Source::YouTube);
    }

    #[test]
    fn without_an_account_every_search_runs_on_youtube() {
        assert_eq!(chosen_source(false, Source::Spotify), Source::YouTube);
        assert_eq!(chosen_source(false, Source::YouTube), Source::YouTube);
    }
}
