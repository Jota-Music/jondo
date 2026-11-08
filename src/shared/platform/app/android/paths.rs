use std::path::PathBuf;

pub fn data_dir() -> Option<PathBuf> {
    gpui_mobile::packages::path_provider::support_directory().ok()
}
