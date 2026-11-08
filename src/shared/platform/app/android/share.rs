//! Handing a file to another app.
//!
//! The share chooser and the package installer both reach a private file
//! through a `content://` URI, because nothing outside the app can read
//! `filesDir`. `Share.java` mints that URI with the `FileProvider` the manifest
//! declares, so both calls are the same string in, string out.

use std::path::Path;

const CLASS: &str = "com/jondo/jondo/Share";
const SEND: &str = "(Landroid/app/Activity;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;";
const INSTALL: &str = "(Landroid/app/Activity;Ljava/lang/String;)Ljava/lang/String;";

/// Opens the share chooser on `path`. The chooser names the app the user
/// picks, so no activity is started behind their back.
pub fn send(path: &Path, mime: &str, title: &str) -> Result<(), String> {
    let path = path.display().to_string();
    let answer =
        crate::shared::platform::android::java::call(CLASS, "send", SEND, &[&path, mime, title])?;
    match answer {
        Some(error) if !error.is_empty() => Err(error),
        _ => Ok(()),
    }
}

/// Hands a downloaded APK to the system package installer. The installer asks
/// the user to confirm; the app is replaced when they do.
pub fn install(path: &Path) -> Result<(), String> {
    let path = path.display().to_string();
    let answer = crate::shared::platform::android::java::call(CLASS, "install", INSTALL, &[&path])?;
    match answer {
        Some(error) if !error.is_empty() => Err(error),
        _ => Ok(()),
    }
}
