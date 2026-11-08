use std::path::{Path, PathBuf};

pub(super) fn dir() -> Result<PathBuf, String> {
    Err("automatic install is not supported on this channel".to_owned())
}

pub(super) fn install(_staged: &Path) -> Result<PathBuf, String> {
    Err("automatic install is not supported on this channel".to_owned())
}
