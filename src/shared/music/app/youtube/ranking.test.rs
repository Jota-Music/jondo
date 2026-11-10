use super::*;

// The candidates below are taken from the music.youtube.com video shelf for the
// query the app builds, in the order YouTube returned them.

fn video(title: &str, duration: u32) -> Video {
    Video {
        id: "id".to_owned(),
        title: title.to_owned(),
        duration,
        ..Default::default()
    }
}

fn first(videos: Vec<Video>, song: &Song) -> Video {
    rank(videos, song)
        .into_iter()
        .next()
        .expect("rank returned nothing")
}

fn assert_not_first(videos: Vec<Video>, song: &Song, title: &str) {
    for (index, video) in rank(videos, song).iter().enumerate() {
        if video.title == title {
            assert_ne!(index, 0, "{title:?} ranked first, want it demoted");
            return;
        }
    }

    panic!("{title:?} not in results");
}

// Havana is 3:39. The top hit is the 6:43 music video with the cameo, and the
// live Grammy cut sits third; the studio audio is the exact length.
#[test]
fn rank_prefers_the_studio_length() {
    let song = Song {
        name: "Havana".to_owned(),
        duration: 219,
        ..Default::default()
    };
    let candidates = vec![
        video(
            "Camila Cabello - Havana (Official Video) ft. Young Thug",
            403,
        ),
        video("Camila Cabello - Havana (Audio) ft. Young Thug", 219),
        video("Camila Cabello - Havana (Lyrics) ft. Young Thug", 213),
        video(
            "Camila Cabello - Havana (Vertical Video) ft. Young Thug",
            217,
        ),
        video("Camila Cabello - Havana (Young Thug only)", 101),
        video("Camila Cabello - Havana (Without Young Thug)", 217),
        video("Camila Cabello - Havana (Live - Audio)", 251),
        video("Havana (LIVE at the 61st GRAMMYs)", 292),
    ];

    assert!(first(candidates, &song).title.contains("(Audio)"));
}

// Reik never published a music video for "Ahora Sin Ti", so YouTube's shelf for
// it is one official upload and a wall of fans. The official one is the band's
// own VEVO channel and the label named it "(Cover Audio)", because Reik released
// someone else's song as a single. Reading "cover" as a version keyword demoted
// the one authoritative upload below a fan re-upload.
#[test]
fn rank_prefers_the_artists_own_cover() {
    let song = Song {
        name: "Ahora Sin Ti".to_owned(),
        duration: 181,
        artists: vec!["Reik".to_owned()],
        ..Default::default()
    };
    let mut candidates = vec![
        video("Reik - Ahora Sin Ti (Cover Audio)", 181),
        video("Ahora Sin Ti letra", 180),
        video("Reik - Ahora sin ti", 188),
        video("Reik - Ahora Sin Ti (Video Oficial) 2018 Estreno", 183),
        video("Reik ahora sin ti LYRICS", 184),
        video("Reik - Ahora Sin Ti", 180),
        video("Ahora Sin Ti", 222),
    ];

    // The app reads the author from shortBylineText, which shortens reikVEVO to
    // "Reik" — the channel still matches the artist exactly.
    candidates[0].author = "Reik".to_owned();
    candidates[1].author = "REIK".to_owned();
    candidates[6].author = "Dorathy Pipes - Topic".to_owned();

    let got = first(candidates, &song);
    assert_eq!(got.title, "Reik - Ahora Sin Ti (Cover Audio)");
}

// A fan's cover shares the title and the exact length of the track, so only the
// uploader gives it away.
#[test]
fn rank_demotes_a_fan_cover() {
    let song = Song {
        name: "Havana".to_owned(),
        duration: 219,
        artists: vec!["Camila Cabello".to_owned()],
        ..Default::default()
    };
    let candidates = vec![
        Video {
            id: "fan".to_owned(),
            title: "Camila Cabello - Havana (cover)".to_owned(),
            author: "Random Covers".to_owned(),
            duration: 219,
            ..Default::default()
        },
        Video {
            id: "master".to_owned(),
            title: "Camila Cabello - Havana".to_owned(),
            author: "Camila Cabello".to_owned(),
            duration: 219,
            ..Default::default()
        },
    ];

    assert_eq!(first(candidates, &song).id, "master");
}

#[test]
fn cover_penalty() {
    assert_eq!(cover("Song (Cover)", 0.0), KEYWORD_PENALTY);

    // The artist's own cover is the master, not a fan re-upload.
    assert_eq!(cover("Song (Cover Audio)", CHANNEL_BONUS), 0.0);
    assert_eq!(cover("Song (Cover) (Tribute)", 0.0), 2.0 * KEYWORD_PENALTY);
}

// A "8D audio", a "lo-fi" flip or a "bass boosted" upload restates the song at
// its own length but is not the original recording, so the plain upload wins.
#[test]
fn rank_demotes_degraded_reuploads() {
    let song = Song {
        name: "Havana".to_owned(),
        duration: 219,
        ..Default::default()
    };
    let candidates = vec![
        video("Camila Cabello - Havana (8D AUDIO)", 219),
        video("Camila Cabello - Havana (slowed + reverb)", 219),
        video("Camila Cabello - Havana (Lo-Fi)", 219),
        video("Camila Cabello - Havana (Bass Boosted)", 219),
        video("Camila Cabello - Havana", 219),
    ];

    assert_eq!(first(candidates, &song).title, "Camila Cabello - Havana");
}

// "Te Quise Olvidar" is MDO's hit, co-written and first recorded by Carlos
// Baute. Only the title says who is singing, so without the credit Baute wins
// on duration alone.
#[test]
fn rank_prefers_the_titled_artist() {
    let song = Song {
        name: "Te Quise Olvidar".to_owned(),
        duration: 263,
        artists: vec!["MDO".to_owned()],
        ..Default::default()
    };
    let candidates = vec![
        Video {
            id: "baute".to_owned(),
            title: "Te Quise Olvidar - Carlos Baute (Audio HQ)".to_owned(),
            author: "Frankjvs".to_owned(),
            duration: 259,
            ..Default::default()
        },
        Video {
            id: "mdo".to_owned(),
            title: "MDO - Te Quise Olvidar (Audio)".to_owned(),
            author: "MDOisback".to_owned(),
            duration: 268,
            ..Default::default()
        },
        Video {
            id: "mdo-video".to_owned(),
            title: "MDO - Te Quise Olvidar (Video Oficial)".to_owned(),
            author: "MdoSME".to_owned(),
            duration: 274,
            ..Default::default()
        },
    ];

    assert_eq!(first(candidates, &song).id, "mdo");
}

#[test]
fn credit_penalty() {
    let song = Song {
        artists: vec!["The Weeknd".to_owned()],
        ..Default::default()
    };
    let titled = Video {
        title: "The Weeknd - Blinding Lights".to_owned(),
        ..Default::default()
    };

    assert_eq!(credit(&titled, &song, 0.0), CREDIT_BONUS);

    // The artist's own channel already carries the trust; the credit must not
    // stack on top of it.
    assert_eq!(credit(&titled, &song, CHANNEL_BONUS), 0.0);

    let other = Video {
        title: "Blinding Lights - Dua Lipa".to_owned(),
        ..Default::default()
    };
    assert_eq!(credit(&other, &song, 0.0), 0.0);
    assert_eq!(
        credit(&video("Blinding Lights (Audio)", 0), &song, 0.0),
        0.0
    );
}

// The Voice performance ranks first for "Fast Car": a different artist at half
// the length.
#[test]
fn rank_rejects_a_mismatched_length() {
    let song = Song {
        name: "Fast Car".to_owned(),
        duration: 296,
        ..Default::default()
    };
    let candidates = vec![
        video(
            "Ceri Hall Brady - Fast Car | The Voice 2022 (Germany) | All Around The Voice",
            130,
        ),
        video(
            "Black Pumas - Fast Car (Tracy Chapman Cover) (Live on The Tonight Show)",
            282,
        ),
        video("Jonas Blue - Fast Car ft. Dakota (Official Video)", 216),
        video("LethalBizzle - Fast Car", 182),
        video("Tracy Chapman - Fast Car (Official Music Video)", 267),
        video("Tracy Chapman - Fast Car (Live)", 342),
    ];

    assert_not_first(
        candidates,
        &song,
        "Ceri Hall Brady - Fast Car | The Voice 2022 (Germany) | All Around The Voice",
    );
}

// Searching SULTANESSA returns only other Freya Ridings songs. The correct track
// is not in the results, so ranking cannot fix the pick, but it must not hand
// back a confident same-artist mismatch while a title shares no word at all.
#[test]
fn rank_demotes_a_different_song() {
    let song = Song {
        name: "SULTANESSA".to_owned(),
        duration: 178,
        ..Default::default()
    };
    let candidates = vec![
        video("Freya Ridings - Castles", 209),
        video("Freya Ridings - Castles (Live At The Barbican)", 199),
        video("Freya Ridings - Castles (Official Lyric Video)", 212),
        video("Freya Ridings - Undefeated (Official Lyric Video)", 263),
        video("Freya Ridings - Wicker Woman (Official Video)", 192),
        video("Freya Ridings - Euphoria (Official Lyric Video)", 172),
    ];

    assert_not_first(candidates, &song, "Freya Ridings - Castles");
}

#[test]
fn duration_gap_needs_both_lengths() {
    // An unknown length on either side leaves the duration out of the score
    // rather than treating the video as a mismatch.
    assert_eq!(duration_gap(0, 219), 0.0);
    assert_eq!(duration_gap(403, 0), 0.0);
    assert_eq!(duration_gap(219, 219), 0.0);
    assert!(duration_gap(403, 219) > duration_gap(219, 219));
}

#[test]
fn missing_tokens() {
    let song = Song {
        name: "Regreso al sexo químicamente puro (Remastered 2011)".to_owned(),
        ..Default::default()
    };

    assert_eq!(
        missing("Ilegales - Regreso al sexo quimicamente puro", &song),
        0.0
    );
    assert_eq!(
        missing("Ilegales - Regreso (Official Video)", &song),
        3.0 * TOKEN_PENALTY
    );

    // A title made only of stopwords and noise leaves nothing to compare, so it
    // must not penalise every candidate equally.
    let empty = Song {
        name: "The (Official)".to_owned(),
        ..Default::default()
    };
    assert_eq!(missing("The Official Video", &empty), 0.0);

    // Spotify hangs the remaster year onto the title with a dash, outside the
    // brackets, and no upload repeats it.
    let remaster = Song {
        name: "Blinding Lights - Remastered 2024".to_owned(),
        ..Default::default()
    };
    assert_eq!(
        missing("The Weeknd - Blinding Lights (Official Audio)", &remaster),
        0.0
    );
    assert_eq!(
        missing("The Weeknd - Save Your Tears", &remaster),
        2.0 * TOKEN_PENALTY
    );
}

// A subtitled lyric re-upload runs within a second of the studio length, so
// only the upload itself gives it away: the title carries the marker and the
// channel is a random name. The official remaster has to win.
#[test]
fn rank_prefers_the_official_upload() {
    let song = Song {
        name: "Bohemian Rhapsody".to_owned(),
        duration: 354,
        artists: vec!["Queen".to_owned()],
        ..Default::default()
    };
    let candidates = vec![
        Video {
            id: "a".to_owned(),
            title: "Queen - Bohemian Rhapsody (Sub. Español + Lyrics)".to_owned(),
            author: "sweetblue.".to_owned(),
            duration: 355,
            ..Default::default()
        },
        Video {
            id: "b".to_owned(),
            title: "Queen – Bohemian Rhapsody (Official Video Remastered HD)".to_owned(),
            author: "Queen Official".to_owned(),
            duration: 360,
            ..Default::default()
        },
        Video {
            id: "c".to_owned(),
            title: "Queen - Bohemian Rhapsody (Live Aid 1985)".to_owned(),
            author: "Live Aid and Queen Official".to_owned(),
            duration: 165,
            ..Default::default()
        },
        Video {
            id: "d".to_owned(),
            title: "Queen - Bohemian Rhapsody (Karaoke Version)".to_owned(),
            author: "Sing King".to_owned(),
            duration: 375,
            ..Default::default()
        },
        Video {
            id: "e".to_owned(),
            title: "Benson Boone with Special Guest Brian May - Bohemian Rhapsody".to_owned(),
            author: "Benson Boone".to_owned(),
            duration: 374,
            ..Default::default()
        },
    ];

    assert_eq!(first(candidates, &song).author, "Queen Official");
}

#[test]
fn version_penalty() {
    assert_eq!(
        version("INXS - Kick (Live at Wembley Stadium, 1991)"),
        KEYWORD_PENALTY
    );
    assert_eq!(version("Song (Slowed + Reverb + Nightcore)"), KEYWORD_CAP);

    // Known limitation: the match is on whole words anywhere in the title, so
    // a track with "live" in its own name is treated as a live recording.
    assert_eq!(version("Major Lazer - Live It Up"), KEYWORD_PENALTY);
}

#[test]
fn upload_penalty() {
    let song = Song {
        artists: vec!["Queen".to_owned()],
        ..Default::default()
    };
    let author = |author: &str| Video {
        author: author.to_owned(),
        ..Default::default()
    };

    assert_eq!(upload(&author("Queen Official"), &song), CHANNEL_BONUS);
    assert_eq!(upload(&author("QueenVEVO"), &song), CHANNEL_BONUS);
    assert_eq!(upload(&author("Queen Oficial"), &song), CHANNEL_BONUS);

    // Carrying the artist's name is not the same as being the artist.
    for channel in [
        "WOS DS3",
        "Trueno and MILO J",
        "Rosalia Fandom",
        "Khea Mohale",
    ] {
        let name = channel.split_whitespace().next().unwrap();
        let artist = Song {
            artists: vec![name.to_owned()],
            ..Default::default()
        };
        assert_eq!(upload(&author(channel), &artist), 0.0, "{channel:?}");
    }

    // The official artist badge is YouTube's own declaration, so it settles the
    // channel even when the byline cannot be read as the artist's own name.
    let badged = Video {
        author: "OficialLuisMiguel".to_owned(),
        official: true,
        ..Default::default()
    };
    assert_eq!(upload(&badged, &song), CHANNEL_BONUS);

    // The badge outranks the lyric words.
    let badged_lyric = Video {
        title: "Queen - Song (Lyrics Video)".to_owned(),
        official: true,
        ..Default::default()
    };
    assert_eq!(upload(&badged_lyric, &song), CHANNEL_BONUS);

    // A topic channel is trustworthy but not the artist.
    assert_eq!(upload(&author("Queen - Topic"), &song), TOPIC_BONUS);
    assert_eq!(upload(&author("Queen - Temas"), &song), TOPIC_BONUS);
    assert_eq!(upload(&author("7clouds"), &song), 0.0);

    // The marker counts in the title too, under a channel with no tell.
    let subtitle = Video {
        title: "Queen - Song (Sub. Español + Lyrics)".to_owned(),
        author: "sweetblue.".to_owned(),
        ..Default::default()
    };
    assert_eq!(upload(&subtitle, &song), LYRIC_PENALTY);

    // A lyric video wins over its own uploader: the artist posting one is still
    // a lyric video, and its running time is the video's.
    let own = Video {
        title: "Queen - Song (Official Lyric Video)".to_owned(),
        author: "Queen Official".to_owned(),
        ..Default::default()
    };
    assert_eq!(upload(&own, &song), LYRIC_PENALTY);

    // The lyric tier has to sink an exact length.
    let exact = Video {
        title: "Song (Lyrics)".to_owned(),
        author: "Letras y Letra".to_owned(),
        duration: 219,
        ..Default::default()
    };
    let plain = Song {
        name: "Song".to_owned(),
        duration: 219,
        ..Default::default()
    };
    assert!(penalty(&exact, &plain) > 0.0);
}

// The artist posting a different take is still the artist, but the tier breaks
// ties and cannot argue with a length that is a quarter off.
#[test]
fn tier_does_not_outweigh_a_length_gap() {
    let song = Song {
        name: "Yesterday".to_owned(),
        duration: 175,
        artists: vec!["The Beatles".to_owned()],
        ..Default::default()
    };
    let candidates = vec![
        Video {
            id: "a".to_owned(),
            title: "Yesterday (Remastered 2009)".to_owned(),
            author: "The Beatles".to_owned(),
            duration: 126,
            ..Default::default()
        },
        Video {
            id: "b".to_owned(),
            title: "The Beatles - Yesterday".to_owned(),
            author: "The Beatles".to_owned(),
            duration: 175,
            ..Default::default()
        },
    ];

    assert_eq!(first(candidates, &song).id, "b");
    assert_eq!(trust(CHANNEL_BONUS, 0.02), CHANNEL_BONUS);
    assert_eq!(trust(CHANNEL_BONUS, 0.5), 0.0);
    assert_eq!(trust(LYRIC_PENALTY, 0.5), LYRIC_PENALTY);
}

#[test]
fn tokens_of_a_noisy_title() {
    assert_eq!(
        tokens("The Weeknd - Blinding Lights (Official Music Video) [4K]"),
        vec!["weeknd", "blinding", "lights", "music"]
    );
}

// YouTube Music answers the query with the artist's own channel under a byline
// ("OficialLuisMiguel") the suffix matcher cannot read, while a re-upload names
// the artist in its title and matches the track's length exactly. Only the
// official artist badge tells them apart.
#[test]
fn rank_prefers_the_badged_official() {
    let song = Song {
        name: "El tiempo que te quede libre".to_owned(),
        duration: 146,
        artists: vec!["Luis Miguel".to_owned()],
        ..Default::default()
    };
    let candidates = vec![
        Video {
            id: "official".to_owned(),
            title: "El tiempo que te quede libre".to_owned(),
            author: "OficialLuisMiguel".to_owned(),
            duration: 147,
            official: true,
            ..Default::default()
        },
        Video {
            id: "reupload".to_owned(),
            title: "LUIS MIGUEL - El tiempo que te quede libre".to_owned(),
            author: "Crispy Duck".to_owned(),
            duration: 146,
            ..Default::default()
        },
    ];

    assert_eq!(first(candidates, &song).id, "official");
}

// Without a badge the topic channel is the official signal, and naming the
// artist in a stranger's title must not tie it.
#[test]
fn topic_beats_title_credit() {
    let song = Song {
        name: "Song".to_owned(),
        duration: 200,
        artists: vec!["Artist".to_owned()],
        ..Default::default()
    };
    let candidates = vec![
        Video {
            id: "topic".to_owned(),
            title: "Song".to_owned(),
            author: "Artist - Topic".to_owned(),
            duration: 204,
            ..Default::default()
        },
        Video {
            id: "reupload".to_owned(),
            title: "Artist - Song".to_owned(),
            author: "Some Channel".to_owned(),
            duration: 200,
            ..Default::default()
        },
    ];

    assert_eq!(first(candidates, &song).id, "topic");
}

#[test]
fn queries_follow_the_album_and_the_artist() {
    let song = Song {
        name: "Lo que quieras".to_owned(),
        artists: vec!["Rey".to_owned()],
        album: "Noche".to_owned(),
        ..Default::default()
    };

    assert_eq!(
        queries(&song),
        vec!["Lo que quieras Rey Noche", "Rey Lo que quieras"]
    );

    let plain = Song {
        name: "Havana".to_owned(),
        ..Default::default()
    };
    assert_eq!(queries(&plain), vec!["Havana"]);
}
