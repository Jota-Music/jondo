use std::time::{Duration, Instant};

use crate::music::rooms::wire;
use crate::music::rooms::{self, Role, Status};
use crate::shared::music::domain::Song;
use crate::shared::ui::views::design::color::hex;

use crate::music::player::{parse_repeat, repeat_name, resolve};
use crate::shell::app::{Event, Jondo};

impl Jondo {
    pub(crate) fn dial(&mut self, role: Role, code: String, cx: &mut gpui::Context<Self>) {
        let relay = self.rooms.read(cx).relay.trim().to_owned();
        let token = self.rooms.read(cx).token.trim().to_owned();
        let password = self.rooms.read(cx).password.clone();
        // Every dial starts from a clean session, the way Jota's `reset` does:
        // a stale epoch, queue or half-open round from the previous room must
        // never leak into the new connection.
        self.rooms.update(cx, |rooms, _| rooms.reset_session());
        self.rooms.update(cx, |rooms, _| rooms.code = code.clone());
        self.rooms.update(cx, |rooms, _| rooms.error = None);
        self.rooms.update(cx, |rooms, _| rooms.last_role = role);
        self.rooms.update(cx, |rooms, _| rooms.role = role);
        self.rooms
            .update(cx, |rooms, _| rooms.status = Status::Connecting);

        let url = match rooms::relay::ws_url(&relay, &code, role.wire()) {
            Ok(url) => url,
            Err(error) => {
                self.rooms
                    .update(cx, |rooms, _| rooms.status = Status::Idle);
                self.rooms.update(cx, |rooms, _| rooms.error = Some(error));
                return;
            }
        };

        self.rooms.update(cx, |rooms, _| rooms.connection += 1);
        self.rooms.update(cx, |rooms, _| rooms.status_at = None);
        let generation = self.rooms.read(cx).connection;
        let (outbound, commands) = tokio::sync::mpsc::unbounded_channel();
        self.rooms
            .update(cx, |rooms, _| rooms.outbound = Some(outbound));
        self.library_changed(cx);

        let emit = self.jobs.emit();
        self.jobs.run(rooms::relay::run(
            generation,
            url,
            token,
            password,
            commands,
            move |event| emit(Event::Room(Box::new(event))),
        ));
    }

    pub(crate) fn send(&self, cx: &gpui::App, value: serde_json::Value) {
        rooms::relay::send(&self.rooms.read(cx).outbound, value.to_string());
    }

    pub(crate) fn frame(&self, kind: &str, playback: wire::Playback) -> serde_json::Value {
        let mut value = serde_json::to_value(playback).unwrap_or_default();
        if let Some(object) = value.as_object_mut() {
            object.insert("t".to_owned(), serde_json::Value::String(kind.to_owned()));
        }

        value
    }

    pub(crate) fn control(&self, cx: &gpui::App, value: serde_json::Value) {
        if !self.rooms.read(cx).live() {
            return;
        }

        self.send(cx, value);
    }

    pub(crate) fn request_join(&mut self, cx: &mut gpui::Context<Self>) {
        self.rooms
            .update(cx, |rooms, _| rooms.join_at = rooms::now_ms());
        self.rooms
            .update(cx, |rooms, _| rooms.join_sent = Some(Instant::now()));
        self.send(
            cx,
            serde_json::json!({ "t": "join", "at": self.rooms.read(cx).join_at }),
        );
        let _ = cx;
    }

    pub(crate) fn playback(&self, cx: &gpui::App) -> wire::Playback {
        let player = self.player.read(cx);
        let song = player.current();
        let position = player.seek_preview.unwrap_or(player.progress);
        wire::Playback {
            song_id: song.map(|song| song.id.clone()).unwrap_or_default(),
            song: song.map(wire::WireSong::from),
            youtube_id: song.and_then(|song| song.video().map(str::to_owned)),
            index: player.queue.at,
            shuffle: Some(player.queue.shuffle),
            repeat: Some(repeat_name(player.repeat).to_owned()),
            color: Some(hex(self.accent)),
            binary: Some(hex(self.binary)),
            playing: player.playing,
            position_ms: (position * 1000.0).round(),
            at: None,
            generation: None,
            epoch: None,
        }
    }

    pub(crate) fn broadcast_queue(&mut self, cx: &mut gpui::Context<Self>) {
        if !self.rooms.read(cx).live() || !self.rooms.read(cx).joined {
            return;
        }

        let (songs, revision) = {
            let player = self.player.read(cx);
            (player.queue.songs.clone(), player.queue.revision)
        };
        self.rooms
            .update(cx, |rooms, _| rooms.last_queue = songs.clone());
        self.rooms
            .update(cx, |rooms, _| rooms.queue_revision = revision);
        let data = serde_json::to_string(&wire::songs(&songs)).unwrap_or_default();
        self.send(cx, serde_json::json!({ "t": "queue", "data": data }));
    }

    pub(crate) fn propagate_queue(&mut self, cx: &mut gpui::Context<Self>) {
        if !self.rooms.read(cx).joined || !self.rooms.read(cx).live() {
            return;
        }

        let revision = self.player.read(cx).queue.revision;
        if revision == self.rooms.read(cx).queue_revision {
            return;
        }

        let songs = self.player.read(cx).queue.songs.clone();

        // Jota sends one added song or one resolved video as a control op, so a
        // peer keeps its queue object instead of rebuilding the list; anything
        // structural still travels whole and refreshes the relay's join cache.
        match crate::music::player::domain::queue::delta(&self.rooms.read(cx).last_queue, &songs) {
            Some(crate::music::player::domain::queue::Edit::Enqueue { songs, at }) => {
                self.control(
                    cx,
                    serde_json::json!({
                        "action": "enqueue",
                        "songs": wire::songs(&songs),
                        "at": at,
                    }),
                );
            }
            Some(crate::music::player::domain::queue::Edit::Video { id, video }) => {
                self.control(
                    cx,
                    serde_json::json!({
                        "action": "youtube",
                        "id": id,
                        "youtubeId": video,
                    }),
                );
            }
            None => {
                self.broadcast_queue(cx);
                return;
            }
        }

        self.rooms.update(cx, |rooms, _| rooms.last_queue = songs);
        self.rooms
            .update(cx, |rooms, _| rooms.queue_revision = revision);
    }

    pub(crate) fn apply_queue(&mut self, items: Vec<wire::WireSong>, cx: &mut gpui::Context<Self>) {
        let songs = wire::unwrap_songs(items);
        let (current, at) = {
            let player = self.player.read(cx);
            (
                player.current().map(|song| song.id.clone()),
                player.queue.at,
            )
        };
        // A queue replacement that no longer holds the current track keeps the
        // row instead of jumping to the top, the way Jota clamps its index.
        let index = current
            .and_then(|id| songs.iter().position(|song| song.id == id))
            .unwrap_or_else(|| at.min(songs.len().saturating_sub(1)));
        self.player.update(cx, |player, _| {
            player.queue.replace(songs, index);
            player.duration = player
                .current()
                .map(|song| song.duration as f64)
                .unwrap_or(0.0);
            if player.current().is_none() {
                player.stop();
            }
        });

        let (songs, revision) = {
            let player = self.player.read(cx);
            (player.queue.songs.clone(), player.queue.revision)
        };
        self.rooms.update(cx, |rooms, _| rooms.last_queue = songs);
        self.rooms
            .update(cx, |rooms, _| rooms.queue_revision = revision);
        self.library_changed(cx);
    }

    pub(crate) fn start_round(&mut self, cx: &mut gpui::Context<Self>) {
        let Some(song) = self.player.read(cx).current().cloned() else {
            return;
        };

        let generation = rooms::hex(16);
        self.rooms
            .update(cx, |rooms, _| rooms.pending_gen = Some(generation.clone()));
        self.rooms.update(cx, |rooms, _| rooms.prepared = None);
        self.rooms.update(cx, |rooms, _| rooms.release = None);
        self.rooms.update(cx, |rooms, _| rooms.pending_start = true);
        self.rooms.update(cx, |rooms, _| {
            rooms.round_deadline = Some(Instant::now() + Duration::from_secs(8))
        });
        self.player.update(cx, |player, _| {
            player.loading = true;
            player.error = None;
        });
        self.broadcast_queue(cx);
        let (at, shuffle, repeat) = {
            let player = self.player.read(cx);
            (
                player.queue.at,
                player.queue.shuffle,
                repeat_name(player.repeat),
            )
        };
        let payload = serde_json::json!({
            "t": "prepare",
            "gen": generation,
            "epoch": self.rooms.read(cx).epoch,
            "songId": song.id,
            "song": wire::WireSong::from(&song),
            "youtubeId": song.video(),
            "index": at,
            "shuffle": shuffle,
            "repeat": repeat,
            "color": hex(self.accent),
            "binary": hex(self.binary),
        });
        self.send(cx, payload);
        self.load_round(song, generation, cx);
    }

    pub(crate) fn load_round(
        &mut self,
        song: Song,
        generation: String,
        cx: &mut gpui::Context<Self>,
    ) {
        self.rooms.update(cx, |rooms, _| rooms.load_token += 1);
        let token = self.rooms.read(cx).load_token;
        let youtube = self.youtube.clone();
        let known = self.store.youtube_links.get(&song.id).cloned();
        self.jobs.spawn(async move {
            let (song, result) = resolve(youtube, song, known).await;
            Event::Room(Box::new(rooms::Event::Prepared {
                token,
                generation,
                song,
                result,
            }))
        });
    }

    pub(crate) fn handle_prepare(
        &mut self,
        playback: wire::Playback,
        cx: &mut gpui::Context<Self>,
    ) {
        let Some(song) = playback.song.clone().map(Song::from) else {
            return;
        };

        self.player.update(cx, |player, _| {
            player.queue.at = playback
                .index
                .min(player.queue.songs.len().saturating_sub(1));
            if let Some(on) = playback.shuffle {
                player.queue.flag_shuffle(on);
            }

            if let Some(mode) = playback.repeat.as_deref()
                && let Some(repeat) = parse_repeat(mode)
            {
                player.repeat = repeat;
            }

            match player
                .queue
                .songs
                .iter()
                .position(|item| item.id == song.id)
            {
                Some(index) => player.queue.at = index,
                None => {
                    let index = playback.index.min(player.queue.songs.len());
                    player.queue.insert(vec![song.clone()], index);
                    player.queue.at = index.min(player.queue.songs.len().saturating_sub(1));
                }
            }

            if let Some(video) = playback.youtube_id.as_deref() {
                player.queue.update_video(&song.id, video);
            }
        });

        self.library_changed(cx);
        let generation = playback
            .generation
            .clone()
            .filter(|generation| !generation.is_empty())
            .unwrap_or_else(|| rooms::hex(16));
        self.rooms
            .update(cx, |rooms, _| rooms.pending_gen = Some(generation.clone()));
        self.rooms.update(cx, |rooms, _| rooms.prepared = None);
        self.rooms.update(cx, |rooms, _| rooms.release = None);
        self.rooms.update(cx, |rooms, _| rooms.pending_start = true);
        self.rooms.update(cx, |rooms, _| {
            rooms.round_deadline = Some(Instant::now() + Duration::from_secs(8))
        });
        self.player.update(cx, |player, _| player.loading = true);
        self.load_round(song, generation, cx);
    }

    pub(crate) fn handle_play(
        &mut self,
        generation: String,
        epoch: String,
        at: i64,
        position_ms: f64,
        cx: &mut gpui::Context<Self>,
    ) {
        if !epoch.is_empty() {
            self.rooms.update(cx, |rooms, _| rooms.epoch = epoch);
        }

        if self.rooms.read(cx).pending_gen.as_deref() != Some(generation.as_str()) {
            return;
        }

        let at = at.min(self.rooms.read(cx).server_now() + 3000);
        self.rooms.update(cx, |rooms, _| {
            rooms.release = Some(rooms::Release {
                generation,
                // A relay clock far ahead must not hold the room open, so the wait
                // is bounded the way Jota's `delayUntil` bounds it.
                at,
                position_ms,
            });
        });
        self.try_release(cx);
    }

    pub(crate) fn try_release(&mut self, cx: &mut gpui::Context<Self>) {
        let Some(release) = self.rooms.read(cx).release.clone() else {
            return;
        };

        if self.rooms.read(cx).prepared.as_deref() != Some(release.generation.as_str()) {
            return;
        }

        if self.rooms.read(cx).server_now() < release.at {
            return;
        }

        let target = rooms::project(
            release.position_ms,
            release.at,
            true,
            self.rooms.read(cx).server_now(),
        );
        self.player.update(cx, |player, _| {
            player.seek(target);
            player.resume();
            player.loading = false;
        });
        self.rooms.update(cx, |rooms, _| rooms.pending_gen = None);
        self.rooms.update(cx, |rooms, _| rooms.prepared = None);
        self.rooms.update(cx, |rooms, _| rooms.release = None);
        self.rooms
            .update(cx, |rooms, _| rooms.pending_start = false);
        self.rooms
            .update(cx, |rooms, _| rooms.round_deadline = None);
        self.library_changed(cx);
    }

    pub(crate) fn handle_sync(&mut self, playback: wire::Playback, cx: &mut gpui::Context<Self>) {
        if !self.rooms.read(cx).live() || !self.rooms.read(cx).joined {
            return;
        }

        let (loading, seek_preview, playing, progress, current) = {
            let player = self.player.read(cx);
            (
                player.loading,
                player.seek_preview.is_some(),
                player.playing,
                player.progress,
                player
                    .current()
                    .map(|song| song.id.clone())
                    .unwrap_or_default(),
            )
        };
        if self.rooms.read(cx).pending_start || loading || seek_preview {
            return;
        }

        if self.rooms.read(cx).last_control.elapsed() < Duration::from_secs(3) {
            return;
        }

        if current != playback.song_id {
            if rooms::now_ms() - self.rooms.read(cx).join_at > 3000 {
                self.request_join(cx);
            }

            return;
        }

        if playback.playing != playing {
            self.player.update(cx, |player, _| {
                if playback.playing {
                    player.resume();
                } else {
                    player.pause();
                }
            });
        }

        let at = playback
            .at
            .unwrap_or_else(|| self.rooms.read(cx).server_now());
        let target = rooms::project(
            playback.position_ms,
            at,
            playback.playing,
            self.rooms.read(cx).server_now(),
        );
        if (target - progress).abs() > 0.3 {
            self.player.update(cx, |player, _| player.seek(target));
        }
    }

    pub(crate) fn handle_control(&mut self, control: wire::Control, cx: &mut gpui::Context<Self>) {
        self.rooms
            .update(cx, |rooms, _| rooms.last_control = Instant::now());
        match control.action.as_str() {
            "play" => self.player.update(cx, |player, _| player.resume()),
            "pause" => self.player.update(cx, |player, _| player.pause()),
            "stop" => self.player.update(cx, |player, _| {
                player.stop();
                player.progress = 0.0;
            }),
            "seek" => self
                .player
                .update(cx, |player, _| player.seek(control.position_ms / 1000.0)),
            "shuffle" => {
                let shuffle = self.player.read(cx).queue.shuffle;
                let on = control.on.unwrap_or(!shuffle);
                self.player
                    .update(cx, |player, _| player.queue.set_shuffle(on, control.seed));
            }
            "repeat" => {
                if let Some(repeat) = control.mode.as_deref().and_then(parse_repeat) {
                    self.player.update(cx, |player, _| player.repeat = repeat);
                }
            }
            "enqueue" => {
                self.player.update(cx, |player, _| {
                    player
                        .queue
                        .insert(wire::unwrap_songs(control.songs), control.at)
                });
                self.sync_room_queue(cx);
                self.library_changed(cx);
            }
            "youtube" => {
                if let (Some(id), Some(video)) =
                    (control.id.as_deref(), control.youtube_id.as_deref())
                {
                    self.player
                        .update(cx, |player, _| player.queue.update_video(id, video));
                    self.sync_room_queue(cx);
                    self.library_changed(cx);
                }
            }
            _ => {}
        }
    }

    fn sync_room_queue(&mut self, cx: &mut gpui::Context<Self>) {
        let (songs, revision) = {
            let player = self.player.read(cx);
            (player.queue.songs.clone(), player.queue.revision)
        };
        self.rooms.update(cx, |rooms, _| rooms.last_queue = songs);
        self.rooms
            .update(cx, |rooms, _| rooms.queue_revision = revision);
    }

    pub(crate) fn apply_snapshot(
        &mut self,
        echo: i64,
        at: i64,
        queue: Option<Vec<wire::WireSong>>,
        state: Option<wire::Playback>,
        cx: &mut gpui::Context<Self>,
    ) {
        let t3 = rooms::now_ms();
        let round_trip = (t3 - self.rooms.read(cx).join_at - (at - echo)).max(0) as u64;
        let offset = ((echo - self.rooms.read(cx).join_at) + (at - t3)) / 2;
        self.rooms
            .update(cx, |rooms, _| rooms.sample(round_trip, offset));
        self.rooms.update(cx, |rooms, _| rooms.joined = true);
        self.rooms.update(cx, |rooms, _| rooms.join_sent = None);

        if self.rooms.read(cx).role == Role::Host {
            self.broadcast_queue(cx);
        } else if let Some(queue) = queue.clone() {
            self.apply_queue(queue, cx);
        } else {
            // A room with no cached queue does not inherit ours by accident:
            // remember the local queue as already sent so the first tick does
            // not push a whole snapshot into the room.
            self.sync_room_queue(cx);
        }

        if self.rooms.read(cx).role == Role::Host && self.player.read(cx).current().is_some() {
            let empty = state
                .as_ref()
                .map(|state| state.song_id.is_empty() && state.song.is_none())
                .unwrap_or(true);
            if empty {
                let playback = self.playback(cx);
                self.send(cx, self.frame("state", playback));
                self.rooms
                    .update(cx, |rooms, _| rooms.pending_start = false);
                self.rooms
                    .update(cx, |rooms, _| rooms.round_deadline = None);
                return;
            }
        }

        let Some(state) = state else {
            if queue.is_some() {
                self.player.update(cx, |player, _| {
                    player.stop();
                    player.progress = 0.0;
                });
            }

            self.rooms
                .update(cx, |rooms, _| rooms.pending_start = false);
            self.rooms
                .update(cx, |rooms, _| rooms.round_deadline = None);
            return;
        };

        if self.rooms.read(cx).pending_start {
            return;
        }

        self.player.update(cx, |player, _| {
            if let Some(on) = state.shuffle {
                player.queue.flag_shuffle(on);
            }

            if let Some(mode) = state.repeat.as_deref()
                && let Some(repeat) = parse_repeat(mode)
            {
                player.repeat = repeat;
            }

            player.queue.at = state.index.min(player.queue.songs.len().saturating_sub(1));
            if let (id, Some(video)) = (state.song_id.as_str(), state.youtube_id.as_deref()) {
                player.queue.update_video(id, video);
            }
        });

        let song = state.song.clone().map(Song::from).or_else(|| {
            let id = state.song_id.clone();
            self.player
                .read(cx)
                .queue
                .songs
                .iter()
                .find(|song| song.id == id)
                .cloned()
        });
        let Some(song) = song else {
            self.rooms
                .update(cx, |rooms, _| rooms.pending_start = false);
            self.rooms
                .update(cx, |rooms, _| rooms.round_deadline = None);
            return;
        };

        self.rooms.update(cx, |rooms, _| rooms.pending_start = true);
        self.rooms.update(cx, |rooms, _| {
            rooms.round_deadline = Some(Instant::now() + Duration::from_secs(8))
        });
        self.player.update(cx, |player, _| player.loading = true);
        self.rooms.update(cx, |rooms, _| rooms.load_token += 1);
        let token = self.rooms.read(cx).load_token;
        let playing = state.playing;
        let position_ms = state.position_ms;
        let at = state.at.unwrap_or(at);
        let youtube = self.youtube.clone();
        let known = self.store.youtube_links.get(&song.id).cloned();
        self.jobs.spawn(async move {
            let (song, result) = resolve(youtube, song, known).await;
            Event::Room(Box::new(rooms::Event::Snapshot {
                token,
                song,
                playing,
                position_ms,
                at,
                result,
            }))
        });
    }
}
