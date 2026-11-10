use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// `.../Jondo.app/Contents/MacOS/jondo` -> `.../Jondo.app`. A build started with
/// `cargo run` lives outside a bundle and cannot replace itself.
fn bundle() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|error| error.to_string())?;
    let app = exe
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("app"))
        })
        .ok_or_else(|| "not running from an app bundle".to_owned())?;

    Ok(app.to_path_buf())
}

pub(super) fn native() -> bool {
    bundle().is_ok()
}

/// Stages the download away from the bundle: the archive is unpacked next to it
/// only when the swap runs, so a slow download never sits inside the app.
pub(super) fn dir() -> Result<PathBuf, String> {
    let dir = std::env::temp_dir().join("jondo-updates");
    std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    Ok(dir)
}

pub(super) fn install(staged: &Path) -> Result<PathBuf, String> {
    let app = bundle()?;
    let unpacked = std::env::temp_dir().join(format!("jondo-unpack-{}", std::process::id()));

    // Waits for this process to exit: replacing a bundle while its binary is
    // mapped would swap the folder under the running app.
    let script = r#"while kill -0 "$1" 2>/dev/null; do sleep 0.2; done
rm -rf "$3" "$4"
/usr/bin/ditto -x -k "$2" "$4"
mv "$4/Jondo.app" "$3"
open "$3"
"#;

    Command::new("sh")
        .arg("-c")
        .arg(script)
        .arg("sh")
        .arg(std::process::id().to_string())
        .arg(staged)
        .arg(&app)
        .arg(&unpacked)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| error.to_string())?;

    Ok(app)
}
