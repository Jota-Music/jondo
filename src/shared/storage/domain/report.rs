use serde::{Deserialize, Serialize};

use super::zero;

/// What the last pass read and changed. It is stored so the panel can say what
/// happened, and the write baseline is what the freshness rule compares against.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SyncStatus {
    #[serde(default, skip_serializing_if = "zero")]
    pub at: i64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub uri: String,
    #[serde(default, rename = "writeAt", skip_serializing_if = "zero")]
    pub write_at: i64,
    #[serde(default, skip_serializing_if = "zero")]
    pub hash: u32,
    #[serde(default, rename = "readStamp", skip_serializing_if = "zero")]
    pub read_stamp: i64,
    #[serde(default, rename = "readAt", skip_serializing_if = "zero")]
    pub read_at: i64,
    #[serde(default, rename = "snapshotsAt", skip_serializing_if = "zero")]
    pub snapshots_at: i64,
    #[serde(default, skip_serializing_if = "zero")]
    pub mtime: i64,
    #[serde(default, rename = "fileSize", skip_serializing_if = "zero")]
    pub file_size: i64,
    #[serde(default, skip_serializing_if = "zero")]
    pub bytes: usize,
    #[serde(default, skip_serializing_if = "zero")]
    pub remote: usize,
    #[serde(default, skip_serializing_if = "zero")]
    pub imported: usize,
    #[serde(default, skip_serializing_if = "zero")]
    pub modified: bool,
    #[serde(default, skip_serializing_if = "zero")]
    pub stale: bool,
    /// True when the document carries a session this device cannot open with
    /// the pass it holds, so the panel asks for the right one instead of
    /// staying silent about it.
    #[serde(default, skip_serializing_if = "zero")]
    pub sealed: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub problem: String,
    /// The provider's own name for the document, which is the only readable
    /// label a `content://` location has on the phone.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
}
