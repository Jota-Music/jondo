use std::ffi::c_void;
use std::sync::Mutex;
use std::time::Duration;

use souvlaki::{
    MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig,
    SeekDirection,
};

use super::Command;

static CONTROLS: Mutex<Option<MediaControls>> = Mutex::new(None);
/// MPRIS treats a missing `Seek` amount as a step of its own, and ten seconds
/// is the jump the player bar and every other desktop player use.
const STEP: f64 = 10.0;

pub(super) fn install(window: &gpui::Window) {
    let config = PlatformConfig {
        display_name: "Jondo",
        dbus_name: "jondo",
        hwnd: hwnd(window),
    };

    let mut controls = match MediaControls::new(config) {
        Ok(controls) => controls,
        Err(error) => {
            log::warn!("media controls: {error}");
            return;
        }
    };

    if let Err(error) = controls.attach(event) {
        log::warn!("media controls: {error}");
        return;
    }

    *super::lock(&CONTROLS) = Some(controls);
}

fn event(event: MediaControlEvent) {
    let command = match event {
        MediaControlEvent::Play => Command::Play,
        MediaControlEvent::Pause => Command::Pause,
        MediaControlEvent::Toggle => Command::Toggle,
        MediaControlEvent::Stop => Command::Stop,
        MediaControlEvent::Next => Command::Next,
        MediaControlEvent::Previous => Command::Previous,
        MediaControlEvent::Seek(direction) => Command::Skip(step(direction)),
        MediaControlEvent::SeekBy(direction, amount) => {
            Command::Skip(step(direction).signum() * amount.as_secs_f64())
        }
        MediaControlEvent::SetPosition(position) => Command::Seek(position.0.as_secs_f64()),
        _ => return,
    };

    super::queue(command);
}

fn step(direction: SeekDirection) -> f64 {
    match direction {
        SeekDirection::Forward => STEP,
        SeekDirection::Backward => -STEP,
    }
}

pub(super) fn metadata(
    title: &str,
    artist: &str,
    album: &str,
    duration: Duration,
    artwork: Option<&str>,
) {
    let mut slot = super::lock(&CONTROLS);
    let Some(controls) = slot.as_mut() else {
        return;
    };

    let metadata = MediaMetadata {
        title: Some(title),
        artist: Some(artist),
        album: (!album.is_empty()).then_some(album),
        cover_url: artwork,
        duration: Some(duration),
    };

    if let Err(error) = controls.set_metadata(metadata) {
        log::warn!("media metadata: {error}");
    }
}

pub(super) fn state(playing: bool, position: Duration) {
    let mut slot = super::lock(&CONTROLS);
    let Some(controls) = slot.as_mut() else {
        return;
    };

    let progress = Some(MediaPosition(position));
    let playback = if playing {
        MediaPlayback::Playing { progress }
    } else {
        MediaPlayback::Paused { progress }
    };

    if let Err(error) = controls.set_playback(playback) {
        log::warn!("media state: {error}");
    }
}

pub(super) fn clear() {
    let mut slot = super::lock(&CONTROLS);
    let Some(controls) = slot.as_mut() else {
        return;
    };

    // Stopping hides the panel, and the empty metadata keeps the old title
    // from lingering in shells that keep the last card on screen.
    if let Err(error) = controls.set_playback(MediaPlayback::Stopped) {
        log::warn!("media state: {error}");
    }
    if let Err(error) = controls.set_metadata(MediaMetadata::default()) {
        log::warn!("media metadata: {error}");
    }
}

#[cfg(target_os = "windows")]
fn hwnd(window: &gpui::Window) -> Option<*mut c_void> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let handle = HasWindowHandle::window_handle(window).ok()?;
    match handle.as_raw() {
        RawWindowHandle::Win32(handle) => Some(handle.hwnd.as_ptr()),
        _ => None,
    }
}

#[cfg(not(target_os = "windows"))]
fn hwnd(_window: &gpui::Window) -> Option<*mut c_void> {
    None
}
