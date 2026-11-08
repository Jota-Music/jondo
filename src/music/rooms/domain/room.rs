use serde::Deserialize;

use crate::shared::music::domain::Song;
use crate::shared::music::domain::Stream;

pub use crate::shared::music::domain::Room;

use super::wire;

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomStatus {
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub members: usize,
    #[serde(default)]
    pub has_host: bool,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub state: Option<wire::Playback>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Status {
    #[default]
    Idle,
    Connecting,
    Open,
    Closed,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Role {
    #[default]
    Off,
    Host,
    Guest,
}

impl Role {
    pub fn of(value: &str) -> Self {
        match value {
            "host" => Self::Host,
            "guest" => Self::Guest,
            _ => Self::Off,
        }
    }

    pub fn wire(self) -> &'static str {
        match self {
            Self::Host => "host",
            Self::Guest => "guest",
            Self::Off => "",
        }
    }

    pub fn active(self) -> bool {
        self != Self::Off
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TokenState {
    #[default]
    None,
    Required,
    Accepted,
    Rejected,
}

#[derive(Clone, Debug, Default)]
pub struct RelayCheck {
    pub error: Option<String>,
    pub token: TokenState,
}

pub enum Event {
    Connected(u64),
    Closed(u64, bool),
    Message(u64, String),
    Reconnect(u64),
    Status(String, Result<RoomStatus, String>),
    /// The relay and token the verdict belongs to, so a check that finished
    /// after the field changed cannot land on the new value.
    Check(String, String, RelayCheck),
    Prepared {
        token: u64,
        generation: String,
        song: Song,
        result: Result<Stream, String>,
    },
    Snapshot {
        token: u64,
        song: Song,
        playing: bool,
        position_ms: f64,
        at: i64,
        result: Result<Stream, String>,
    },
}

#[derive(Clone, Debug)]
pub struct Release {
    pub generation: String,
    pub at: i64,
    pub position_ms: f64,
}
