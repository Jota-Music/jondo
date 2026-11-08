use serde::{Deserialize, Serialize};

/// A saved room, as the store keeps it and the backup carries it.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Room {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub relay_url: String,
    #[serde(default)]
    pub token: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub saved_at: u64,
}

/// The identity of a saved room: the relay plus its code, so the same room at
/// the same relay is one entry however it was typed.
pub fn room_id(relay: &str, code: &str) -> String {
    format!("{}|{}", relay.trim().to_lowercase(), code.trim())
}
