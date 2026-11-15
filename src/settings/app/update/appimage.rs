use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};

use crate::shared::platform::relaunch::after_exit;

/// Whether this process runs from an AppImage, the only Linux build that can
/// replace itself.
pub(super) fn native() -> bool {
    std::env::var_os("APPIMAGE").is_some_and(|path| !path.is_empty())
}

/// The running AppImage, following symlinks so an update replaces the real
/// file and not the link that points at it.
fn appimage() -> Result<PathBuf, String> {
    let path = std::env::var_os("APPIMAGE")
        .filter(|path| !path.is_empty())
        .ok_or_else(|| "not running from an AppImage".to_owned())?;
    let path = PathBuf::from(path);
    Ok(path.canonicalize().unwrap_or(path))
}

/// Stages the download next to the running AppImage so the final swap is a
/// same-filesystem rename.
pub(super) fn dir() -> Result<PathBuf, String> {
    let target = appimage()?;
    target
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "the AppImage has no folder".to_owned())
}

pub(super) fn install(staged: &Path) -> Result<PathBuf, String> {
    let target = appimage()?;
    std::fs::set_permissions(staged, std::fs::Permissions::from_mode(0o755))
        .map_err(|error| error.to_string())?;
    std::fs::rename(staged, &target).map_err(|error| format!("Replace AppImage: {error}"))?;
    after_exit(&target).map_err(|error| format!("Installed, reopen Jondo ({error})"))?;
    Ok(target)
}
