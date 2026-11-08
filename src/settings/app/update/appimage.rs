use std::os::unix::fs::PermissionsExt as _;
use std::os::unix::process::CommandExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

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
    relaunch(&target).map_err(|error| format!("Installed, reopen Jondo ({error})"))?;
    Ok(target)
}

/// Waits for this process to exit, then execs the new AppImage. Starting it
/// while this one is alive would only focus the running instance.
fn relaunch(path: &Path) -> std::io::Result<()> {
    let script = r#"while kill -0 "$1" 2>/dev/null; do sleep 0.2; done; exec "$2""#;
    Command::new("sh")
        .arg("-c")
        .arg(script)
        .arg("sh")
        .arg(std::process::id().to_string())
        .arg(path)
        .process_group(0)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
}
