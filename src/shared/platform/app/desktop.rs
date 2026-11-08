#[cfg(not(target_os = "android"))]
use std::path::PathBuf;

/// A native dialog blocks until the user answers, so it runs on a worker
/// thread: the portal owns the window and the UI keeps painting behind it.
#[cfg(not(target_os = "android"))]
pub(crate) fn dialog(open: impl FnOnce(rfd::FileDialog) -> Option<PathBuf>) -> Option<PathBuf> {
    open(rfd::FileDialog::new())
}

#[cfg(not(target_os = "android"))]
pub(crate) fn downloads() -> Option<std::path::PathBuf> {
    directories::UserDirs::new()
        .and_then(|dirs| dirs.download_dir().map(|path| path.to_path_buf()))
        .or_else(crate::shared::storage::dir)
}
