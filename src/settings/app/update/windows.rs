use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The per-user folder the NSIS installer writes to. A build run from `target`
/// is not installed and cannot update itself.
fn installed() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|error| error.to_string())?;
    let local = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .ok_or_else(|| "no LOCALAPPDATA".to_owned())?;
    let root = local.join("Programs").join("Jondo");

    exe.starts_with(&root)
        .then_some(exe)
        .ok_or_else(|| "not an installed build".to_owned())
}

pub(super) fn native() -> bool {
    installed().is_ok()
}

pub(super) fn dir() -> Result<PathBuf, String> {
    let dir = std::env::temp_dir().join("jondo-updates");
    std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    Ok(dir)
}

/// Runs the per-user installer silently once this process has exited, then
/// opens the installed build again.
pub(super) fn install(staged: &Path) -> Result<PathBuf, String> {
    let exe = installed()?;
    let script = std::env::temp_dir().join(format!("jondo-update-{}.cmd", std::process::id()));

    // The installer cannot overwrite a binary Windows still has loaded, so the
    // brief wait is the point: the process has to be gone before `/S` runs.
    let body = format!(
        "@echo off\r\nping -n 2 127.0.0.1 >nul\r\nstart \"\" /wait \"{}\" /S\r\nstart \"\" \"{}\"\r\n",
        staged.display(),
        exe.display()
    );
    std::fs::write(&script, body).map_err(|error| error.to_string())?;

    Command::new("cmd")
        .args(["/C", script.to_string_lossy().as_ref()])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| error.to_string())?;

    Ok(exe)
}
