use std::sync::Arc;

use crate::music::player::domain::queue::Repeat;
use crate::shared::i18n::t;
use crate::shared::music::domain::Song;
use crate::shared::youtube::{Stream, Youtube};

pub(crate) fn external_playlist(id: &str) -> bool {
    (id.starts_with("spotify:user:") && id.ends_with(":collection"))
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

pub(crate) async fn resolve(
    youtube: Arc<Youtube>,
    mut song: Song,
    known: Option<String>,
) -> (Song, Result<Stream, String>) {
    let fixed = song.video().filter(|id| !id.is_empty()).map(str::to_owned);
    let linked = known.filter(|id| !id.is_empty());

    for candidate in [fixed.clone(), linked] {
        let Some(video) = candidate else {
            continue;
        };

        match youtube.stream(&video).await {
            Ok(stream) => {
                song.video_id = Some(video);
                return (song, Ok(stream));
            }
            Err(error) if fixed.is_some() => return (song, Err(error)),
            Err(_) => {}
        }
    }

    let video = match youtube.search(&song.query()).await {
        Ok(videos) => match videos.first() {
            Some(video) => {
                song.video_id = Some(video.id.clone());
                video.id.clone()
            }
            None => return (song, Err(t("No YouTube match").to_owned())),
        },
        Err(error) => return (song, Err(error)),
    };

    let result = youtube.stream(&video).await;
    (song, result)
}
