use std::time::Instant;

use crate::music::track::app::editor::{SWITCH, Slide};

use super::parse_video;

#[test]
fn reads_every_video_link_shape() {
    let id = "dQw4w9WgXcQ";
    for text in [
        id,
        "youtube:dQw4w9WgXcQ",
        "https://www.youtube.com/watch?v=dQw4w9WgXcQ&list=PL123",
        "https://music.youtube.com/watch?v=dQw4w9WgXcQ",
        "https://youtu.be/dQw4w9WgXcQ",
        "https://m.youtube.com/shorts/dQw4w9WgXcQ",
        "https://www.youtube.com/embed/dQw4w9WgXcQ",
        "https://www.youtube.com/live/dQw4w9WgXcQ",
    ] {
        assert_eq!(parse_video(text).as_deref(), Some(id), "{text}");
    }
}

#[test]
fn the_mode_slide_resumes_from_where_it_is_and_settles_on_its_target() {
    let now = Instant::now();
    let mut slide = Slide::default();
    assert_eq!(slide.value(now), 0.0);

    slide.start(1.0, now);
    assert_eq!(slide.value(now), 0.0);
    assert_eq!(slide.value(now + SWITCH), 1.0);
    assert!(!slide.running(now + SWITCH));

    slide.start(0.0, now + SWITCH);
    assert_eq!(slide.value(now + SWITCH), 1.0);
    assert_eq!(slide.value(now + SWITCH * 2), 0.0);
    assert!(!slide.running(now + SWITCH * 2));
}

#[test]
fn rejects_links_without_a_video() {
    for text in [
        "",
        "abc",
        "https://www.youtube.com/playlist?list=PL1234567890",
        "https://open.spotify.com/track/4cOdK2wGLETKBW3PvgPWqT",
    ] {
        assert_eq!(parse_video(text), None, "{text}");
    }
}
