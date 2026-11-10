use std::collections::VecDeque;
use std::sync::Mutex;

use gpui_mobile::packages::media_session::{self, MediaAction};

use crate::shell::app::Jondo;

static ACTIONS: Mutex<VecDeque<MediaAction>> = Mutex::new(VecDeque::new());
static SEEK: Mutex<Option<f64>> = Mutex::new(None);
static PUSHED: Mutex<Option<(String, String, u64, bool)>> = Mutex::new(None);
static STATE: Mutex<Option<(bool, u64)>> = Mutex::new(None);
static PREFS: Mutex<Option<(bool, bool)>> = Mutex::new(None);

/// The notification shade, the lock screen and headphone buttons all go through
/// the system media session, so the app has to answer with the same controls
/// the player bar offers.
pub fn install() {
    if let Err(error) = media_session::init() {
        log::warn!("media session: {error}");
    }

    media_session::set_action_handler(|action| {
        if let Ok(mut queue) = ACTIONS.lock() {
            queue.push_back(action);
        }
    });
    media_session::set_seek_handler(|millis| {
        if let Ok(mut slot) = SEEK.lock() {
            *slot = Some(millis as f64 / 1000.0);
        }
    });
}

/// Called from the frame tick: applies what the system controls asked for and
/// republishes the current track.
pub fn sync(app: &mut Jondo, cx: &mut gpui::Context<Jondo>) {
    apply(app);

    let actions: Vec<MediaAction> = match ACTIONS.lock() {
        Ok(mut queue) => queue.drain(..).collect(),
        Err(_) => Vec::new(),
    };

    for action in actions {
        match action {
            MediaAction::Play if !app.player.read(cx).playing => app.toggle(cx),
            MediaAction::Pause | MediaAction::Stop if app.player.read(cx).playing => app.toggle(cx),
            MediaAction::Next => app.next(cx),
            MediaAction::Previous => app.previous(cx),
            _ => {}
        }
    }

    if let Ok(mut slot) = SEEK.lock()
        && let Some(seconds) = slot.take()
    {
        app.seek(seconds, cx);
    }

    publish(app, cx);
}

fn apply(app: &Jondo) {
    let prefs = (app.store.pause_on_disconnect, app.store.pause_on_focus);
    let changed = match PREFS.lock() {
        Ok(mut last) => {
            if last.as_ref() == Some(&prefs) {
                false
            } else {
                *last = Some(prefs);
                true
            }
        }
        Err(_) => false,
    };
    if !changed {
        return;
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

fn publish(app: &Jondo, cx: &gpui::App) {
    let player = app.player.read(cx);
    let Some(song) = player.current() else {
        return;
    };

    let artist = song.artists.join(", ");
    let artwork = song
        .cover
        .as_deref()
        .and_then(crate::shared::cover::path)
        .filter(|path| path.exists());

    let metadata = (
        song.id.clone(),
        artist.clone(),
        (player.duration.max(0.0) * 1000.0) as u64,
        artwork.is_some(),
    );
    let changed = match PUSHED.lock() {
        Ok(mut pushed) => {
            if pushed.as_ref() == Some(&metadata) {
                false
            } else {
                *pushed = Some(metadata.clone());
                true
            }
        }
        Err(_) => false,
    };

    if changed {
        if let Err(error) = media_session::set_metadata(&song.name, &artist, metadata.2) {
            log::warn!("media session metadata: {error}");
        }

        if let Some(path) = &artwork
            && let Err(error) = media_session::set_artwork(&path.display().to_string())
        {
            log::warn!("media session artwork: {error}");
        }
    }

    // The system extrapolates the position from the playback speed, so a
    // whole-second key is enough: comparing milliseconds would push a JNI
    // call on every frame of the playback clock.
    let position = (player.progress.max(0.0) * 1000.0) as u64;
    let state = (
        player.playing,
        if player.playing {
            position / 1000
        } else {
            position
        },
    );
    let advanced = match STATE.lock() {
        Ok(mut pushed) => {
            if pushed.as_ref() == Some(&state) {
                false
            } else {
                *pushed = Some(state);
                true
            }
        }
        Err(_) => false,
    };

    if advanced
        && let Err(error) = media_session::set_playback_state(
            state.0,
            if state.0 { state.1 * 1000 } else { state.1 },
            if state.0 { 1.0 } else { 0.0 },
        )
    {
        log::warn!("media session state: {error}");
    }
}
