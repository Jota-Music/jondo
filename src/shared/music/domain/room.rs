use serde::{Deserialize, Serialize};

/// A saved room, as the store keeps it and the backup carries it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Room {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub code: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub relay_url: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub token: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub password: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub saved_at: u64,
}

fn is_zero(value: &u64) -> bool {
    *value == 0
}

/// The identity of a saved room: the relay plus its code, so the same room at
/// the same relay is one entry however it was typed.
pub fn room_id(relay: &str, code: &str) -> String {
    format!("{}|{}", relay.trim().to_lowercase(), code.trim())
}
