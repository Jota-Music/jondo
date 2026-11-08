use serde::{Deserialize, Serialize};

/// What the last pass read and changed. It is stored so the panel can say what
/// happened, and the write baseline is what the freshness rule compares against.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SyncStatus {
    #[serde(default)]
    pub at: i64,
    #[serde(default)]
    pub uri: String,
    #[serde(default, rename = "writeAt")]
    pub write_at: i64,
    #[serde(default)]
    pub hash: u32,
    #[serde(default, rename = "readStamp")]
    pub read_stamp: i64,
    #[serde(default, rename = "readAt")]
    pub read_at: i64,
    #[serde(default, rename = "snapshotsAt")]
    pub snapshots_at: i64,
    #[serde(default)]
    pub mtime: i64,
    #[serde(default, rename = "fileSize")]
    pub file_size: i64,
    #[serde(default)]
    pub bytes: usize,
    #[serde(default)]
    pub remote: usize,
    #[serde(default)]
    pub imported: usize,
    #[serde(default)]
    pub modified: bool,
    #[serde(default)]
    pub stale: bool,
    /// True when the document carries a session this device cannot open with
    /// the pass it holds, so the panel asks for the right one instead of
    /// staying silent about it.
    #[serde(default)]
    pub sealed: bool,
    #[serde(default)]
    pub problem: String,
    /// The provider's own name for the document, which is the only readable
    /// label a `content://` location has on the phone.
    #[serde(default)]
    pub name: String,
}
