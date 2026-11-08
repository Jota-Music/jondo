mod document;
mod format;
mod playlists;
mod sections;
mod stage;
mod sync;
#[cfg(test)]
#[path = "backup.test.rs"]
mod tests;

pub use stage::{committed, failed, restore_staged, stage, stage_dir};
#[cfg(not(target_os = "android"))]
pub(crate) use sync::settled;
pub use sync::{
    Mode, Stats, SyncStatus, auto, export, import, overwrite, pending, restore, save,
    settled_document, stamp, touch,
};

pub const FILE: &str = "jota.backup.json";
