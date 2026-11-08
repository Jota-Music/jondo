use std::time::{Duration, Instant};

use gpui::{Context, Window};

use crate::music::rooms::{self, Role, Status};
use crate::shared::i18n::t;
use crate::shell::app::{Event, Jondo};

impl Jondo {
    pub(crate) fn refresh_room_statuses(&mut self, cx: &mut gpui::Context<Self>) {
        if !self.rooms.read(cx).pending_statuses.is_empty() {
            return;
        }

        self.rooms
            .update(cx, |rooms, _| rooms.status_at = Some(Instant::now()));
        if self.rooms.read(cx).token_required && self.rooms.read(cx).token.trim().is_empty() {
            return;
        }

        if self.store.rooms.is_empty() {
            return;
        }

        let locked = self.rooms.read(cx).relay_locked;
        let global_relay = self.rooms.read(cx).relay.trim().to_owned();
        let global_token = self.rooms.read(cx).token.trim().to_owned();
        for room in self.store.rooms.clone() {
            let relay = if locked || room.relay_url.trim().is_empty() {
                global_relay.clone()
            } else {
                room.relay_url.clone()
            };

            let token = if locked {
                global_token.clone()
            } else {
                room.token.clone()
            };

            if relay.trim().is_empty() {
                continue;
            }

            let id = room.id.clone();
            let code = room.code.clone();
            let http = self.http.clone();
            self.rooms.update(cx, |rooms, _| {
                rooms.pending_statuses.insert(id.clone());
            });
            self.jobs.spawn(async move {
                let result = rooms::relay::status(&http, &relay, &code, &token).await;
                Event::Room(Box::new(rooms::Event::Status(id, result)))
            });
        }
    }

    pub fn ensure_room_statuses(&mut self, cx: &mut gpui::Context<Self>) {
        let stale = self
            .rooms
            .read(cx)
            .status_at
            .map(|at| at.elapsed() > Duration::from_secs(15))
            .unwrap_or(true);
        if stale {
            self.refresh_room_statuses(cx);
        }
    }

    pub fn check_relay(&mut self, cx: &mut gpui::Context<Self>) {
        if self.rooms.read(cx).checking {
            return;
        }

        let relay = self.rooms.read(cx).relay.trim().to_owned();
        let token = self.rooms.read(cx).token.trim().to_owned();
        self.rooms.update(cx, |rooms, _| rooms.check_due = None);
        if relay.is_empty() {
            self.rooms
                .update(cx, |rooms, _| rooms.verdict = rooms::RelayCheck::default());
            self.rooms
                .update(cx, |rooms, _| rooms.token_required = false);
            return;
        }

        self.rooms.update(cx, |rooms, _| rooms.checking = true);
        let http = self.http.clone();
        self.jobs.spawn(async move {
            let verdict = rooms::relay::check(&http, &relay, &token).await;
            Event::Room(Box::new(rooms::Event::Check(relay, token, verdict)))
        });
    }

    pub fn open_rooms(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (code, password) = {
            let rooms = self.rooms.read(cx);
            (rooms.code.clone(), rooms.password.clone())
        };
        if self.rooms.read(cx).draft_code.trim().is_empty() {
            self.rooms.update(cx, |rooms, _| rooms.draft_code = code);
        }

        if self.rooms.read(cx).draft_password.is_empty() {
            self.rooms
                .update(cx, |rooms, _| rooms.draft_password = password);
        }

        self.ui.closing = None;
        self.rooms.update(cx, |rooms, _| rooms.show = true);
        self.sync_room_fields(window, cx);
        self.ensure_room_statuses(cx);
    }

    pub(crate) fn apply_room(&mut self, room: &rooms::Room, cx: &mut gpui::Context<Self>) {
        if !self.rooms.read(cx).relay_locked {
            let keeps = room.relay_url.trim() == self.rooms.read(cx).relay.trim()
                && room.token.trim().is_empty();
            self.rooms
                .update(cx, |rooms, _| rooms.relay = room.relay_url.clone());
            if !keeps {
                self.rooms
                    .update(cx, |rooms, _| rooms.token = room.token.clone());
            }
        }

        self.rooms
            .update(cx, |rooms, _| rooms.draft_code = room.code.clone());
        self.rooms
            .update(cx, |rooms, _| rooms.draft_password = room.password.clone());
        self.rooms.update(cx, |rooms, _| rooms.error = None);
    }

    pub fn edit_room(&mut self, room: &rooms::Room, window: &mut Window, cx: &mut Context<Self>) {
        self.apply_room(room, cx);
        self.ui.closing = None;
        self.rooms.update(cx, |rooms, _| rooms.show = true);
        self.sync_room_fields(window, cx);
    }

    /// The room inputs are created at boot; opening the panel writes the
    /// current draft into them instead of letting render create or edit them.
    fn sync_room_fields(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let code = self.rooms.read(cx).draft_code.clone();
        let password = self.rooms.read(cx).draft_password.clone();
        self.set_field("room-code", &code, window, cx);
        self.set_field("room-password", &password, window, cx);
    }

    pub fn join_room(&mut self, room: &rooms::Room, cx: &mut gpui::Context<Self>) {
        self.apply_room(room, cx);
        self.connect_room(cx);
    }

    pub fn saved_room(&self, code: &str, cx: &gpui::App) -> Option<&rooms::Room> {
        let id = rooms::room_id(&self.rooms.read(cx).relay, code);
        self.store.rooms.iter().find(|room| room.id == id)
    }

    pub fn saved_password(&self, code: &str, cx: &gpui::App) -> String {
        self.saved_room(code, cx)
            .map(|room| room.password.clone())
            .unwrap_or_default()
    }

    pub fn connect_room(&mut self, cx: &mut gpui::Context<Self>) {
        let code = self.rooms.read(cx).draft_code.trim().to_owned();
        if code.is_empty() || self.rooms.read(cx).relay.trim().is_empty() {
            return;
        }

        let password = self.rooms.read(cx).draft_password.trim().to_owned();
        self.rooms.update(cx, |rooms, _| rooms.password = password);
        self.rooms.update(cx, |rooms, _| rooms.retrying = false);
        self.dial(Role::Off, code, cx);
    }

    pub fn stop_room(&mut self, cx: &mut gpui::Context<Self>) {
        self.rooms.update(cx, |rooms, _| {
            rooms.connection += 1;
            rooms.outbound = None;
            rooms.retrying = false;
            rooms.reset_session();
            rooms.status = Status::Idle;
            rooms.status_at = None;
            rooms.code.clear();
            rooms.password.clear();
            rooms.draft_password.clear();
        });
        self.persist(cx);
    }

    pub fn save_room(&mut self, cx: &mut gpui::Context<Self>) {
        let code = self.rooms.read(cx).draft_code.trim().to_owned();
        let relay = self.rooms.read(cx).relay.trim().to_owned();
        if code.is_empty() || relay.is_empty() {
            return;
        }

        let existing = rooms::room_id(&relay, &code);
        let token = if self.rooms.read(cx).relay_locked
            && let Some(room) = self.store.rooms.iter().find(|room| room.id == existing)
        {
            room.token.clone()
        } else {
            self.rooms.read(cx).token.trim().to_owned()
        };

        self.store.save_room(rooms::Room {
            id: String::new(),
            code,
            relay_url: relay,
            token,
            password: self.rooms.read(cx).draft_password.trim().to_owned(),
            name: String::new(),
            saved_at: 0,
        });
        self.rooms.update(cx, |rooms, _| rooms.status_at = None);
        self.persist(cx);
        self.notify(t("Saved to Rooms"));
    }

    pub fn unsave_room(&mut self, id: &str, cx: &mut gpui::Context<Self>) {
        self.store.remove_room(id);
        self.persist(cx);
    }

    pub fn forget_room(&mut self, id: &str, cx: &mut gpui::Context<Self>) {
        if id == rooms::room_id(&self.rooms.read(cx).relay, &self.rooms.read(cx).code) {
            self.stop_room(cx);
        }

        self.unsave_room(id, cx);
    }

    pub fn save_current_room(&mut self, cx: &mut gpui::Context<Self>) {
        if self.rooms.read(cx).code.trim().is_empty() {
            return;
        }

        let (code, password) = {
            let rooms = self.rooms.read(cx);
            (rooms.code.clone(), rooms.password.clone())
        };
        self.rooms.update(cx, |rooms, _| {
            rooms.draft_code = code;
            rooms.draft_password = password;
        });
        self.save_room(cx);
    }

    pub fn commit_room_password(&mut self, cx: &mut gpui::Context<Self>) {
        let relay = self.rooms.read(cx).relay.trim().to_owned();
        let code = self.rooms.read(cx).draft_code.trim().to_owned();
        if code.is_empty() {
            return;
        }

        let id = rooms::room_id(&relay, &code);
        let password = self.rooms.read(cx).draft_password.trim().to_owned();
        if let Some(room) = self.store.rooms.iter_mut().find(|room| room.id == id) {
            room.password = password;
            self.persist(cx);
        }
    }

    pub fn reorder_rooms(&mut self, from: &str, to: &str, cx: &mut gpui::Context<Self>) {
        let mut ids: Vec<String> = self
            .store
            .rooms
            .iter()
            .map(|room| room.id.clone())
            .collect();
        let mut remaining = ids.clone();
        let mut ordered = Vec::with_capacity(remaining.len());
        for id in &self.store.room_order {
            if let Some(position) = remaining.iter().position(|item| item == id) {
                ordered.push(remaining.remove(position));
            }
        }

        ordered.extend(remaining);
        ids = ordered;
        let Some(from_index) = ids.iter().position(|id| id == from) else {
            return;
        };

        let Some(to_index) = ids.iter().position(|id| id == to) else {
            return;
        };

        let id = ids.remove(from_index);
        ids.insert(to_index, id);
        self.store.room_order = ids;
        self.store.room_at = crate::shared::kernel::app::clock::now_ms();
        self.library_changed(cx);
    }
}
