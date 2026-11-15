use std::collections::VecDeque;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use crate::shell::app::Jondo;

#[cfg(target_os = "android")]
mod android;
#[cfg(not(target_os = "android"))]
mod desktop;

#[cfg(target_os = "android")]
use android as backend;
#[cfg(not(target_os = "android"))]
use desktop as backend;

/// What an OS control asked the player to do. Every platform maps its own
/// events into this list, so the player answers them in one place.
pub(super) enum Command {
    Play,
    Pause,
    Toggle,
    Stop,
    Next,
    Previous,
    /// Absolute position, in seconds.
    Seek(f64),
    /// Relative jump, in seconds.
    Skip(f64),
}

#[derive(Clone, PartialEq, Eq)]
struct Push {
    title: String,
    artist: String,
    album: String,
    duration: u64,
    artwork: Option<String>,
}

static COMMANDS: Mutex<VecDeque<Command>> = Mutex::new(VecDeque::new());
static PUSHED: Mutex<Option<Push>> = Mutex::new(None);
static STATE: Mutex<Option<(bool, u64)>> = Mutex::new(None);
static CLEARED: Mutex<bool> = Mutex::new(true);

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

pub(super) fn queue(command: Command) {
    lock(&COMMANDS).push_back(command);
}

/// The notification shade, the lock screen and the media keys all go through
/// the system session, so the app has to answer with the same controls the
/// player bar offers.
#[cfg(target_os = "android")]
pub fn install() {
    backend::install();
}

#[cfg(not(target_os = "android"))]
pub fn install(window: &gpui::Window) {
    backend::install(window);
}

/// Called from the frame tick: applies what the system controls asked for and
/// republishes the current track.
pub fn sync(app: &mut Jondo, cx: &mut gpui::Context<Jondo>) {
    #[cfg(target_os = "android")]
    backend::preferences(app.store.pause_on_disconnect, app.store.pause_on_focus);

    dispatch(app, cx);
    publish(app, cx);
}

fn dispatch(app: &mut Jondo, cx: &mut gpui::Context<Jondo>) {
    let commands = lock(&COMMANDS).drain(..).collect::<Vec<_>>();

    for command in commands {
        match command {
            Command::Play => {
                let player = app.player.read(cx);
                // A load already heading to play needs nothing; a paused one
                // only drops the intent, and a stopped track starts over.
                let resume = player.paused || (!player.playing && !player.loading);
                if resume {
                    app.toggle(cx);
                }
            }
            Command::Pause => {
                let player = app.player.read(cx);
                // Pausing a stream that is still resolving must not restart
                // it: the load carries on with the pause intent set.
                let pause = (player.playing || player.loading) && !player.paused;
                if pause {
                    app.toggle(cx);
                }
            }
            Command::Toggle => app.toggle(cx),
            Command::Stop => app.stop(cx),
            Command::Next => app.next(cx),
            Command::Previous => app.previous(cx),
            Command::Seek(seconds) => app.seek(seconds, cx),
            Command::Skip(delta) => {
                let at = app.player.read(cx).progress + delta;
                app.seek(at, cx);
            }
        }
    }
}

fn publish(app: &Jondo, cx: &gpui::App) {
    let player = app.player.read(cx);
    let Some(song) = player.current() else {
        clear();
        return;
    };

    // The restored track is published before it is loaded, exactly like Jota
    // does, so the desktop still has a player to send the keys to. Only an
    // explicit stop removes the card; a stream being fetched keeps the old one
    // until the new metadata arrives.
    if player.stopped && !player.loading {
        clear();
        return;
    }

    let artist = song.artists.join(", ");
    // The cover travels as the URL the source gave: Jota hands the same link to
    // the system, and no image is ever written to the user's disk.
    let artwork = song.cover.clone();
    let duration = if player.duration > 0.0 {
        player.duration
    } else {
        song.duration as f64
    };
    let push = Push {
        title: song.name.clone(),
        artist: artist.clone(),
        album: song.album.clone(),
        duration: (duration.max(0.0) * 1000.0) as u64,
        artwork: artwork.clone(),
    };

    let changed = {
        let mut pushed = lock(&PUSHED);
        if pushed.as_ref() == Some(&push) {
            false
        } else {
            *pushed = Some(push.clone());
            true
        }
    };

    if changed {
        backend::metadata(
            &push.title,
            &push.artist,
            &push.album,
            Duration::from_millis(push.duration),
            artwork.as_deref(),
        );
        *lock(&CLEARED) = false;
    }

    // The system extrapolates the position from the playback speed, so a
    // whole-second key is enough: comparing milliseconds would push a call on
    // every frame of the playback clock.
    let position = (player.progress.max(0.0) * 1000.0) as u64;
    let state = (
        player.playing,
        if player.playing {
            position / 1000
        } else {
            position
        },
    );
    let advanced = {
        let mut last = lock(&STATE);
        if last.as_ref() == Some(&state) {
            false
        } else {
            *last = Some(state);
            true
        }
    };

    if advanced {
        let millis = if state.0 { state.1 * 1000 } else { state.1 };
        backend::state(state.0, Duration::from_millis(millis));
    }
}

fn clear() {
    let mut cleared = lock(&CLEARED);
    if *cleared {
        return;
    }

    *cleared = true;
    backend::clear();
    *lock(&PUSHED) = None;
    *lock(&STATE) = None;
}
