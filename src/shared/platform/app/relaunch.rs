//! Starting another program once this one is gone.

use std::os::unix::process::CommandExt as _;
use std::path::Path;
use std::process::{Command, Stdio};

/// Waits for this process to exit, then execs `path`. Starting it while this
/// one is alive would only focus the running instance, and a store written on
/// the way out has to reach the disk first.
pub fn after_exit(path: &Path) -> std::io::Result<()> {
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
