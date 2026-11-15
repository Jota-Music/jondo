use std::sync::Mutex;
use std::time::Duration;

use gpui_mobile::packages::media_session::{self, MediaAction};

use super::Command;

static LIVE: Mutex<bool> = Mutex::new(false);
static PREFS: Mutex<Option<(bool, bool)>> = Mutex::new(None);

pub(super) fn install() {
    live(true);
    media_session::set_action_handler(|action| {
        super::queue(match action {
            MediaAction::Play => Command::Play,
            MediaAction::Pause => Command::Pause,
            MediaAction::Stop => Command::Stop,
            MediaAction::Next => Command::Next,
            MediaAction::Previous => Command::Previous,
        });
    });
    media_session::set_seek_handler(|millis| {
        super::queue(Command::Seek(millis as f64 / 1000.0));
    });
}

/// The user's interruption preferences live in the Java session: it owns the
/// audio focus and the headset broadcast.
pub(super) fn preferences(pause_on_disconnect: bool, pause_on_focus: bool) {
    let prefs = (pause_on_disconnect, pause_on_focus);
    {
        let mut last = super::lock(&PREFS);
        if last.as_ref() == Some(&prefs) {
            return;
        }
        *last = Some(prefs);
    }

    let flag = |on: bool| if on { "1" } else { "0" };
    if let Err(error) = crate::shared::platform::android::java::call(
        "com/jondo/jondo/MainActivity",
        "preferences",
        "(Landroid/app/Activity;Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;",
        &[flag(prefs.0), flag(prefs.1)],
    ) {
        log::warn!("media preferences: {error}");
    }
}

pub(super) fn metadata(
    title: &str,
    artist: &str,
    album: &str,
    duration: Duration,
    artwork: Option<&str>,
) {
    live(false);
    if let Err(error) =
        media_session::set_metadata(title, artist, album, duration.as_millis() as u64)
    {
        log::warn!("media session metadata: {error}");
    }

    if let Some(url) = artwork
        && let Err(error) = media_session::set_artwork(url)
    {
        log::warn!("media session artwork: {error}");
    }
}

pub(super) fn state(playing: bool, position: Duration) {
    live(false);
    let speed = if playing { 1.0 } else { 0.0 };
    if let Err(error) =
        media_session::set_playback_state(playing, position.as_millis() as u64, speed)
    {
        log::warn!("media session state: {error}");
    }
}

pub(super) fn clear() {
    if std::mem::replace(&mut *super::lock(&LIVE), false) {
        if let Err(error) = media_session::release() {
            log::warn!("media session release: {error}");
        }
    }
}

/// A stop releases the Java session, so the next track has to start it again.
/// The Java `init` is a no-op while a session is alive.
fn live(fresh: bool) {
    let mut live = super::lock(&LIVE);
    if fresh || !*live {
        if let Err(error) = media_session::init() {
            log::warn!("media session: {error}");
            return;
        }
        *live = true;
    }
}
