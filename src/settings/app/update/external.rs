use std::path::{Path, PathBuf};

/// A build that did not arrive as an artifact this app can replace.
pub(super) fn native() -> bool {
    false
}

pub(super) fn dir() -> Result<PathBuf, String> {
    Err("automatic install is not supported on this channel".to_owned())
}

pub(super) fn install(_staged: &Path) -> Result<PathBuf, String> {
    Err("automatic install is not supported on this channel".to_owned())
}
