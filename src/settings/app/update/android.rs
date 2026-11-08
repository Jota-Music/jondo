use std::path::{Path, PathBuf};

use crate::shared::i18n::t;

/// Stages the APK in the app's private files dir so the Java `FileProvider`
/// can expose it to the system package installer.
pub(super) fn dir() -> Result<PathBuf, String> {
    let dir = crate::shared::storage::dir()
        .map(|dir| dir.join("updates"))
        .ok_or_else(|| t("No folder to save the update").to_owned())?;
    std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    Ok(dir)
}

pub(super) fn install(staged: &Path) -> Result<PathBuf, String> {
    let target = dir()?.join("jondo.apk");
    std::fs::rename(staged, &target).map_err(|error| format!("Stage APK: {error}"))?;
    Ok(target)
}
