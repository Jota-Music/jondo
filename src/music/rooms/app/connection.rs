use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use tokio::sync::mpsc::UnboundedSender;

use crate::shared::music::domain::Song;

use super::relay;
use crate::music::rooms::domain::{RelayCheck, Release, Role, RoomStatus, Status};
use crate::music::rooms::now_ms;

pub struct Rooms {
    pub show: bool,
    pub relay: String,
    pub token: String,
    pub relay_locked: bool,
    pub code: String,
    pub password: String,
    pub draft_code: String,
    pub draft_password: String,
    pub status: Status,
    pub role: Role,
    pub peers: usize,
    pub joined: bool,
    /// A close that arrives after an established connection is retried; one
    /// from the first dial reports the failure instead of spinning forever.
    pub retrying: bool,
    pub error: Option<String>,
    pub token_required: bool,
    pub verdict: RelayCheck,
    pub checking: bool,
    pub check_due: Option<Instant>,
    pub checked_at: Option<Instant>,
    pub statuses: HashMap<String, RoomStatus>,
    pub pending_statuses: HashSet<String>,
    pub status_at: Option<Instant>,
    pub connection: u64,
    pub last_role: Role,
    pub reconnect_delay: u64,
    pub offset_ms: i64,
    pub samples: Vec<(u64, i64)>,
    pub ping_id: u64,
    pub last_ping: Instant,
    pub last_state: Instant,
    pub last_sync: Instant,
    pub last_control: Instant,
    pub join_at: i64,
    pub join_sent: Option<Instant>,
    pub epoch: String,
    /// The queue the last snapshot or edit left on the wire, so a local change
    /// can travel as an incremental edit instead of rewriting the whole list.
    pub last_queue: Vec<Song>,
    pub queue_revision: u64,
    pub pending_gen: Option<String>,
    pub prepared: Option<String>,
    pub pending_start: bool,
    pub release: Option<Release>,
    pub round_deadline: Option<Instant>,
    pub load_token: u64,
    pub members_seen: usize,
    pub outbound: Option<UnboundedSender<relay::Command>>,
}

impl Rooms {
    pub fn new(relay: String, token: String, code: String, password: String) -> Self {
        let now = Instant::now();
        Self {
            show: false,
            relay,
            token,
            relay_locked: false,
            draft_code: code.clone(),
            draft_password: password.clone(),
            code,
            password,
            status: Status::Idle,
            role: Role::Off,
            peers: 0,
            joined: false,
            retrying: false,
            error: None,
            token_required: false,
            verdict: RelayCheck::default(),
            checking: false,
            check_due: None,
            checked_at: None,
            statuses: HashMap::new(),
            pending_statuses: HashSet::new(),
            status_at: None,
            connection: 0,
            last_role: Role::Off,
            reconnect_delay: 0,
            offset_ms: 0,
            samples: Vec::new(),
            ping_id: 0,
            last_ping: now,
            last_state: now,
            last_sync: now,
            last_control: now - Duration::from_secs(10),
            join_at: 0,
            join_sent: None,
            epoch: String::new(),
            last_queue: Vec::new(),
            queue_revision: 0,
            pending_gen: None,
            prepared: None,
            pending_start: false,
            release: None,
            round_deadline: None,
            load_token: 0,
            members_seen: 0,
            outbound: None,
        }
    }

    pub fn active(&self) -> bool {
        self.role.active()
    }

    pub fn live(&self) -> bool {
        self.status == Status::Open && self.role.active()
    }

    pub fn connecting(&self) -> bool {
        matches!(self.status, Status::Connecting | Status::Closed)
    }

    pub fn server_now(&self) -> i64 {
        now_ms() + self.offset_ms
    }

    pub fn sample(&mut self, round_trip: u64, offset: i64) {
        self.samples.push((round_trip, offset));
        if self.samples.len() > 12 {
            self.samples.remove(0);
        }

        let best = self
            .samples
            .iter()
            .min_by_key(|(round_trip, _)| *round_trip);
        if let Some((_, offset)) = best {
            self.offset_ms = *offset;
        }
    }

    pub fn schedule_check(&mut self) {
        self.check_due = Some(Instant::now() + Duration::from_millis(500));
    }

    pub fn reset_session(&mut self) {
        self.role = Role::Off;
        self.peers = 0;
        self.joined = false;
        self.error = None;
        self.pending_gen = None;
        self.prepared = None;
        self.pending_start = false;
        self.release = None;
        self.round_deadline = None;
        self.epoch.clear();
        self.members_seen = 0;
        self.last_queue.clear();
        self.queue_revision = 0;
        self.offset_ms = 0;
        self.samples.clear();
        self.join_sent = None;
    }

    pub fn abandon(&mut self) {
        let error = self.error.clone();
        self.connection += 1;
        self.outbound = None;
        self.reset_session();
        self.error = error;
    }
}
