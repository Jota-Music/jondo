use std::path::PathBuf;
use std::sync::{Condvar, Mutex, OnceLock};

use crate::shared::storage::domain::*;

static WRITER: OnceLock<Slot> = OnceLock::new();
static WRITER_STARTED: OnceLock<()> = OnceLock::new();
static HOME: OnceLock<PathBuf> = OnceLock::new();

/// Pins the folder the app writes to. Called once, before anything reads the
/// store: a test hands it a temporary folder so a run never touches the real
/// state, and `JONDO_HOME` gives the same thing to a portable install.
pub fn use_home(dir: impl Into<PathBuf>) {
    let _ = HOME.set(dir.into());
}

pub fn use_home_from_env() {
    if let Some(dir) = std::env::var_os("JONDO_HOME").filter(|value| !value.is_empty()) {
        use_home(dir);
    }
}

/// The newest store waiting for the writer. Keeping one slot instead of a queue
/// means a burst of edits settles into a single clone instead of one per edit,
/// which on a large library is the difference between one copy and many.
pub(super) struct Slot {
    pub(super) store: Mutex<Option<Store>>,
    pub(super) ready: Condvar,
}

pub(super) fn writer() -> &'static Slot {
    let slot = WRITER.get_or_init(|| Slot {
        store: Mutex::new(None),
        ready: Condvar::new(),
    });
    WRITER_STARTED.get_or_init(|| {
        std::thread::Builder::new()
            .name("jondo-store".to_owned())
            .spawn(move || {
                loop {
                    let Some(store) = take(slot) else {
                        return;
                    };

                    store.write();
                }
            })
            .ok();
    });
    slot
}

pub(super) fn take(slot: &'static Slot) -> Option<Store> {
    let mut waiting = slot.store.lock().ok()?;
    while waiting.is_none() {
        waiting = slot.ready.wait(waiting).ok()?;
    }

    waiting.take()
}

pub fn dir() -> Option<PathBuf> {
    if let Some(home) = HOME.get() {
        return Some(home.clone());
    }

    platform_dir()
}

fn platform_dir() -> Option<PathBuf> {
    #[cfg(target_os = "android")]
    {
        crate::shared::platform::android::data_dir()
    }

    #[cfg(not(target_os = "android"))]
    {
        directories::ProjectDirs::from("", "", "jondo").map(|dirs| dirs.config_dir().to_path_buf())
    }
}

pub fn path() -> Option<PathBuf> {
    dir().map(|dir| dir.join("state.json"))
}
