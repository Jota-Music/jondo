use std::sync::Arc;

use crate::music::player::domain::queue::Repeat;
use crate::shared::i18n::t;
use crate::shared::music::domain::Song;
use crate::shared::youtube::{Stream, Youtube, candidates};

pub(crate) fn external_playlist(id: &str) -> bool {
    id.starts_with("spotify:collection:")
        || id.starts_with("spotify:playlist:")
        || id.starts_with("youtube:playlist:")
}

pub(crate) fn repeat_name(repeat: Repeat) -> &'static str {
    match repeat {
        Repeat::Off => "off",
        Repeat::All => "all",
        Repeat::One => "one",
    }
}

pub(crate) fn parse_repeat(value: &str) -> Option<Repeat> {
    match value {
        "off" => Some(Repeat::Off),
        "all" => Some(Repeat::All),
        "one" => Some(Repeat::One),
        _ => None,
    }
}

/// Tries the linked video first, then every search hit best match first, and
/// only gives up once none of them streams. A candidate that will not play is
/// not the end of the song: it hands the turn to the next match.
pub(crate) async fn resolve(
    youtube: Arc<Youtube>,
    mut song: Song,
    known: Option<String>,
    fresh: bool,
) -> (Song, Result<Stream, String>) {
    let fixed = song.video().map(crate::shared::music::domain::id::video);
    let linked = known
        .filter(|id| !id.is_empty())
        .map(|id| crate::shared::music::domain::id::video(&id));
    let mut failure = None;
    let mut tried = Vec::new();

    for candidate in [fixed, linked].into_iter().flatten() {
        if tried.contains(&candidate) {
            continue;
        }

        match play(&youtube, &candidate, fresh).await {
            Ok(stream) => {
                song.video_id = Some(candidate);
                return (song, Ok(stream));
            }
            Err(error) => {
                failure.get_or_insert(error);
                tried.push(candidate);
            }
        }
    }

    let videos = match candidates(&youtube, &song).await {
        Ok(videos) => videos,
        Err(error) => return (song, Err(error)),
    };

    for video in videos {
        if tried.contains(&video.id) {
            continue;
        }

        match play(&youtube, &video.id, fresh).await {
            Ok(stream) => {
                song.video_id = Some(video.id);
                return (song, Ok(stream));
            }
            Err(error) => {
                failure.get_or_insert(error);
            }
        }
    }

    let error = failure.unwrap_or_else(|| t("No YouTube match").to_owned());
    (song, Err(error))
}

async fn play(youtube: &Youtube, video: &str, fresh: bool) -> Result<Stream, String> {
    if fresh {
        youtube.refresh(video).await
    } else {
        youtube.stream(video).await
    }
}
