use std::time::{Duration, Instant};

use crate::music::rooms::wire;
use crate::music::rooms::{self, Role, Status};
use crate::shared::i18n::t;
use crate::shell::app::{Event, Jondo, Route};

impl Jondo {
    pub(crate) fn room_event(&mut self, event: rooms::Event, cx: &mut gpui::Context<Self>) {
        match event {
            rooms::Event::Connected(generation) => self.room_connected(generation, cx),
            rooms::Event::Closed(generation, required) => {
                self.room_closed(generation, required, cx)
            }
            rooms::Event::Reconnect(generation) => self.room_reconnect(generation, cx),
            rooms::Event::Message(generation, text) => {
                self.room_message_event(generation, &text, cx)
            }
            rooms::Event::Status(id, result) => self.room_status(id, result, cx),
            rooms::Event::Check(relay, token, verdict) => {
                self.room_check(relay, token, verdict, cx)
            }
            rooms::Event::Prepared {
                token,
                generation,
                song,
                result,
            } => self.room_prepared(token, generation, song, result, cx),
            rooms::Event::Snapshot {
                token,
                song,
                playing,
                position_ms,
                at,
                result,
            } => self.room_snapshot(token, song, playing, position_ms, at, result, cx),
        }
    }

    fn room_connected(&mut self, generation: u64, cx: &mut gpui::Context<Self>) {
        if generation != self.rooms.read(cx).connection {
            return;
        }

        self.rooms
            .update(cx, |rooms, _| rooms.status = Status::Open);
        self.rooms.update(cx, |rooms, _| rooms.status_at = None);
        self.rooms.update(cx, |rooms, _| rooms.joined = false);
        self.rooms.update(cx, |rooms, _| rooms.error = None);
        self.rooms.update(cx, |rooms, _| rooms.retrying = false);
        self.rooms.update(cx, |rooms, _| rooms.reconnect_delay = 0);
        self.rooms
            .update(cx, |rooms, _| rooms.last_ping = Instant::now());
        self.rooms
            .update(cx, |rooms, _| rooms.last_state = Instant::now());
        self.rooms
            .update(cx, |rooms, _| rooms.last_sync = Instant::now());
        self.request_join(cx);
        self.rooms.update(cx, |rooms, _| rooms.ping_id += 1);
        let id = self.rooms.read(cx).ping_id;
        let at = rooms::now_ms();
        self.send(cx, serde_json::json!({ "t": "ping", "id": id, "at": at }));
        // Jota probes the clock in a short burst on connect, so a round
        // released right after joining already has a fresh offset.
        let outbound = self.rooms.read(cx).outbound.clone();
        self.rooms.update(cx, |rooms, _| rooms.ping_id += 2);
        let first = self.rooms.read(cx).ping_id - 1;
        let second = self.rooms.read(cx).ping_id;
        self.jobs.run(async move {
            for (delay, id) in [(150u64, first), (250u64, second)] {
                tokio::time::sleep(Duration::from_millis(delay)).await;
                let at = rooms::now_ms();
                rooms::relay::send(
                    &outbound,
                    serde_json::json!({ "t": "ping", "id": id, "at": at }).to_string(),
                );
            }
        });
    }

    fn room_closed(&mut self, generation: u64, required: bool, cx: &mut gpui::Context<Self>) {
        if generation != self.rooms.read(cx).connection {
            return;
        }

        let established = self.rooms.read(cx).status == Status::Open;
        self.rooms.update(cx, |rooms, _| rooms.outbound = None);
        self.rooms.update(cx, |rooms, _| rooms.role = Role::Off);
        self.rooms.update(cx, |rooms, _| rooms.peers = 0);
        self.rooms.update(cx, |rooms, _| rooms.joined = false);
        self.rooms.update(cx, |rooms, _| rooms.pending_gen = None);
        self.rooms.update(cx, |rooms, _| rooms.prepared = None);
        self.rooms
            .update(cx, |rooms, _| rooms.pending_start = false);
        self.rooms
            .update(cx, |rooms, _| rooms.round_deadline = None);
        self.rooms.update(cx, |rooms, _| rooms.release = None);
        self.rooms
            .update(cx, |rooms, _| rooms.status = Status::Closed);
        self.rooms.update(cx, |rooms, _| rooms.status_at = None);
        if required {
            self.rooms
                .update(cx, |rooms, _| rooms.token_required = true);
            self.rooms.update(cx, |rooms, _| {
                rooms.error = Some(t("This relay requires a token.").to_owned())
            });
            self.rooms.update(cx, |rooms, _| rooms.abandon());
            self.rooms
                .update(cx, |rooms, _| rooms.status = Status::Idle);
            return;
        }

        if self.rooms.read(cx).code.is_empty() {
            self.rooms
                .update(cx, |rooms, _| rooms.status = Status::Idle);
            return;
        }

        // A connection that was already open is retried with backoff;
        // a first dial that never reached the relay is a typo or an
        // outage the user has to see, not spin on forever.
        if established {
            self.rooms.update(cx, |rooms, _| rooms.retrying = true);
        }

        if !self.rooms.read(cx).retrying {
            let message =
                t("Could not reach the relay. Check the address and your connection.").to_owned();
            self.rooms
                .update(cx, |rooms, _| rooms.error = Some(message.clone()));
            self.rooms.update(cx, |rooms, _| rooms.abandon());
            self.rooms
                .update(cx, |rooms, _| rooms.status = Status::Idle);
            self.fail(message);
            return;
        }

        let next = if self.rooms.read(cx).reconnect_delay == 0 {
            1000
        } else {
            (self.rooms.read(cx).reconnect_delay * 2).min(15000)
        };
        self.rooms
            .update(cx, |rooms, _| rooms.reconnect_delay = next);

        let delay = self.rooms.read(cx).reconnect_delay + rand::random_range(0..500);
        let emit = self.jobs.emit();
        self.jobs.run(async move {
            tokio::time::sleep(Duration::from_millis(delay)).await;
            emit(Event::Room(Box::new(rooms::Event::Reconnect(generation))));
        });
    }

    fn room_reconnect(&mut self, generation: u64, cx: &mut gpui::Context<Self>) {
        if generation != self.rooms.read(cx).connection
            || self.rooms.read(cx).status != Status::Closed
        {
            return;
        }

        let role = self.rooms.read(cx).last_role;
        self.dial(role, self.rooms.read(cx).code.clone(), cx);
    }

    fn room_message_event(&mut self, generation: u64, text: &str, cx: &mut gpui::Context<Self>) {
        if generation != self.rooms.read(cx).connection {
            return;
        }

        self.room_message(text, cx);
    }

    fn room_status(
        &mut self,
        id: String,
        result: Result<rooms::RoomStatus, String>,
        cx: &mut gpui::Context<Self>,
    ) {
        self.rooms
            .update(cx, |rooms, _| rooms.pending_statuses.remove(&id));
        match result {
            Ok(status) => {
                self.rooms.update(cx, |rooms, _| {
                    rooms.statuses.insert(id, status);
                });
            }
            Err(error) => {
                log::debug!("room status {id}: {error}");
                self.rooms.update(cx, |rooms, _| {
                    rooms.statuses.remove(&id);
                });
            }
        }
    }

    fn room_check(
        &mut self,
        relay: String,
        token: String,
        verdict: rooms::RelayCheck,
        cx: &mut gpui::Context<Self>,
    ) {
        self.rooms.update(cx, |rooms, _| rooms.checking = false);
        if self.rooms.read(cx).relay.trim() != relay || self.rooms.read(cx).token.trim() != token {
            if !self.rooms.read(cx).relay.trim().is_empty() {
                self.rooms.update(cx, |rooms, _| rooms.schedule_check());
            }

            return;
        }

        self.rooms
            .update(cx, |rooms, _| rooms.checked_at = Some(Instant::now()));
        self.rooms.update(cx, |rooms, _| {
            rooms.token_required = verdict.token != rooms::TokenState::None
        });
        self.rooms.update(cx, |rooms, _| rooms.verdict = verdict);
        if let Some(error) = &self.rooms.read(cx).verdict.error {
            log::warn!("relay check: {error}");
        }
    }

    fn room_prepared(
        &mut self,
        token: u64,
        generation: String,
        song: crate::shared::music::domain::Song,
        result: Result<crate::shared::youtube::Stream, String>,
        cx: &mut gpui::Context<Self>,
    ) {
        if token != self.rooms.read(cx).load_token
            || self.rooms.read(cx).pending_gen.as_deref() != Some(generation.as_str())
        {
            return;
        }

        match result {
            Ok(stream) => {
                let linked = song.video().unwrap_or_default().to_owned();
                self.player.update(cx, |player, _| {
                    if let Some(current) = player.queue.current_mut() {
                        current.video_id = song.video_id.clone();
                    }
                    player.prepare(stream, 0.0);
                });

                if self.store.link(&song.id, &linked) {
                    self.save_store();
                }
                self.rooms
                    .update(cx, |rooms, _| rooms.prepared = Some(generation.clone()));
                self.send(
                    cx,
                    serde_json::json!({
                        "t": "ready",
                        "gen": generation,
                        "ok": true,
                    }),
                );
                self.try_release(cx);
            }
            Err(error) => {
                self.send(
                    cx,
                    serde_json::json!({
                        "t": "ready",
                        "gen": generation,
                        "ok": false,
                    }),
                );
                self.rooms.update(cx, |rooms, _| rooms.pending_gen = None);
                self.rooms
                    .update(cx, |rooms, _| rooms.pending_start = false);
                self.rooms
                    .update(cx, |rooms, _| rooms.round_deadline = None);
                self.rooms.update(cx, |rooms, _| rooms.release = None);
                self.player.update(cx, |player, _| {
                    player.loading = false;
                    player.fail(error.clone());
                });
                self.fail(error);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn room_snapshot(
        &mut self,
        token: u64,
        song: crate::shared::music::domain::Song,
        playing: bool,
        position_ms: f64,
        at: i64,
        result: Result<crate::shared::youtube::Stream, String>,
        cx: &mut gpui::Context<Self>,
    ) {
        if token != self.rooms.read(cx).load_token {
            return;
        }

        match result {
            Ok(stream) => {
                let linked = song.video().unwrap_or_default().to_owned();
                let target =
                    rooms::project(position_ms, at, playing, self.rooms.read(cx).server_now());
                self.player.update(cx, |player, _| {
                    if let Some(current) = player.queue.current_mut() {
                        current.video_id = song.video_id.clone();
                    }
                    player.prepare(stream, target);
                    player.loading = false;
                });

                if self.store.link(&song.id, &linked) {
                    self.save_store();
                }

                self.rooms
                    .update(cx, |rooms, _| rooms.pending_start = false);
                self.rooms
                    .update(cx, |rooms, _| rooms.round_deadline = None);
                if playing {
                    if self.rooms.read(cx).server_now() < at {
                        self.rooms
                            .update(cx, |rooms, _| rooms.pending_gen = Some(String::new()));
                        self.rooms
                            .update(cx, |rooms, _| rooms.prepared = Some(String::new()));
                        let at = at.min(self.rooms.read(cx).server_now() + 3000);
                        self.rooms.update(cx, |rooms, _| {
                            rooms.release = Some(rooms::Release {
                                generation: String::new(),
                                // Bounded like Jota's `delayUntil`: a relay
                                // clock far ahead cannot stall the join.
                                at,
                                position_ms,
                            });
                        });
                        // Keep the frame clock at full rate until the
                        // release: an idle beat rounds the shared start
                        // up by up to 250ms.
                        self.player.update(cx, |player, _| player.loading = true);
                    } else {
                        self.player.update(cx, |player, _| {
                            player.seek(target);
                            player.resume();
                        });
                    }
                } else {
                    self.player.update(cx, |player, _| player.pause());
                }

                self.library_changed(cx);
            }
            Err(error) => {
                self.rooms
                    .update(cx, |rooms, _| rooms.pending_start = false);
                self.rooms
                    .update(cx, |rooms, _| rooms.round_deadline = None);
                self.player.update(cx, |player, _| {
                    player.loading = false;
                    player.fail(error.clone());
                });
                self.fail(error);
            }
        }
    }

    pub(crate) fn room_message(&mut self, text: &str, cx: &mut gpui::Context<Self>) {
        let message = match serde_json::from_str::<wire::ServerMessage>(text) {
            Ok(message) => message,
            Err(error) => {
                log::warn!("room: invalid message: {error}");
                return;
            }
        };

        match message {
            wire::ServerMessage::Role { role } => {
                self.rooms
                    .update(cx, |rooms, _| rooms.role = Role::of(&role));
                let role = self.rooms.read(cx).role;
                self.rooms.update(cx, |rooms, _| rooms.last_role = role);
            }
            wire::ServerMessage::Error { reason } => {
                self.rooms
                    .update(cx, |rooms, _| rooms.error = Some(rooms::translate(&reason)));
                self.fail(rooms::translate(&reason));
                self.rooms.update(cx, |rooms, _| rooms.abandon());
                self.rooms.update(cx, |rooms, _| rooms.retrying = false);
                self.rooms
                    .update(cx, |rooms, _| rooms.status = Status::Idle);
            }
            wire::ServerMessage::Pong { at, echo, .. } => {
                let now = rooms::now_ms();
                let round_trip = (now - at).max(0) as u64;
                self.rooms.update(cx, |rooms, _| {
                    rooms.sample(round_trip, echo - at - round_trip as i64 / 2);
                });
            }
            wire::ServerMessage::Members { count, epoch } => {
                self.rooms.update(cx, |rooms, _| rooms.peers = count);
                if !epoch.is_empty() {
                    self.rooms.update(cx, |rooms, _| rooms.epoch = epoch);
                }

                if count > self.rooms.read(cx).members_seen
                    && self.rooms.read(cx).joined
                    && self.rooms.read(cx).role == Role::Host
                {
                    self.broadcast_queue(cx);
                }

                self.rooms.update(cx, |rooms, _| rooms.members_seen = count);
            }
            wire::ServerMessage::Join { from, .. } => {
                if self.rooms.read(cx).role == Role::Host && self.rooms.read(cx).live() {
                    self.send(
                        cx,
                        serde_json::json!({
                            "t": "snapshot",
                            "to": from,
                            "state": self.playback(cx),
                        }),
                    );
                }
            }

            wire::ServerMessage::Snapshot {
                echo,
                at,
                queue,
                state,
            } => self.apply_snapshot(echo, at, queue, state, cx),
            wire::ServerMessage::Play {
                generation,
                epoch,
                at,
                position_ms,
            } => self.handle_play(generation, epoch, at, position_ms, cx),
            _ => {
                if !self.rooms.read(cx).live() {
                    return;
                }

                match message {
                    wire::ServerMessage::Prepare(playback) => self.handle_prepare(playback, cx),
                    wire::ServerMessage::Sync(playback) => self.handle_sync(playback, cx),
                    wire::ServerMessage::Control(control) => self.handle_control(control, cx),
                    wire::ServerMessage::Queue { data } => {
                        if let Ok(items) = serde_json::from_str::<Vec<wire::WireSong>>(&data) {
                            self.apply_queue(items, cx);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    pub(crate) fn room_tick(&mut self, cx: &mut gpui::Context<Self>) {
        if let Some(due) = self.rooms.read(cx).check_due
            && Instant::now() >= due
        {
            self.check_relay(cx);
        }

        // The relay caches each room's playback, so a visible room shelf has to
        // keep asking: querying once on open would freeze every tile on whatever
        // was playing when the page appeared. Jota polls every 15s while mounted.
        if self.route == Route::Rooms || self.rooms.read(cx).show {
            self.ensure_room_statuses(cx);
        }

        if self.rooms.read(cx).live() {
            let now = Instant::now();
            if self.rooms.read(cx).pending_start
                && self
                    .rooms
                    .read(cx)
                    .round_deadline
                    .map(|deadline| now >= deadline)
                    .unwrap_or(false)
            {
                self.rooms.update(cx, |rooms, _| rooms.pending_gen = None);
                self.rooms.update(cx, |rooms, _| rooms.prepared = None);
                self.rooms.update(cx, |rooms, _| rooms.release = None);
                self.rooms
                    .update(cx, |rooms, _| rooms.pending_start = false);
                self.rooms
                    .update(cx, |rooms, _| rooms.round_deadline = None);
                self.player.update(cx, |player, _| player.loading = false);
                self.rooms.update(cx, |rooms, _| {
                    rooms.last_control = now - Duration::from_secs(10)
                });
                self.request_join(cx);
            }

            if now.duration_since(self.rooms.read(cx).last_ping) >= Duration::from_secs(1) {
                self.rooms.update(cx, |rooms, _| rooms.last_ping = now);
                self.rooms.update(cx, |rooms, _| rooms.ping_id += 1);
                let id = self.rooms.read(cx).ping_id;
                let at = rooms::now_ms();
                self.send(cx, serde_json::json!({ "t": "ping", "id": id, "at": at }));
            }

            if now.duration_since(self.rooms.read(cx).last_state) >= Duration::from_secs(2) {
                self.rooms.update(cx, |rooms, _| rooms.last_state = now);
                let playback = self.playback(cx);
                self.send(cx, self.frame("state", playback));
            }

            if self.rooms.read(cx).role == Role::Host
                && now.duration_since(self.rooms.read(cx).last_sync) >= Duration::from_secs(5)
                && !self.rooms.read(cx).pending_start
            {
                self.rooms.update(cx, |rooms, _| rooms.last_sync = now);
                let mut playback = self.playback(cx);
                playback.at = Some(self.rooms.read(cx).server_now());
                self.send(cx, self.frame("sync", playback));
            }

            if self.rooms.read(cx).joined {
                self.propagate_queue(cx);
            }

            self.try_release(cx);
        } else if self.rooms.read(cx).status == Status::Open
            && !self.rooms.read(cx).joined
            && self
                .rooms
                .read(cx)
                .join_sent
                .map(|at| at.elapsed() > Duration::from_secs(4))
                .unwrap_or(true)
        {
            self.request_join(cx);
        }
    }
}
