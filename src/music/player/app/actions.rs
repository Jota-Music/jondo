use crate::shared::i18n::t;

use futures_util::StreamExt;

use crate::shared::music::domain::{Playlist, Song, Source};
use crate::shared::storage::links;
use crate::shared::youtube::candidates;
use crate::shell::app::{Event, Jondo};

/// How many tracks in line are resolved in the background, like Jota's
/// PRELOAD_AHEAD: an already-resolved URL keeps playing while YouTube is slow,
/// blocked or rate limited.
const AHEAD: usize = 4;

/// How many of those resolve at once. Each one holds a player answer in memory
/// while it is parsed, so a handful in flight is a few megabytes of JSON.
const PARALLEL: usize = 2;

impl Jondo {
    pub fn play_playlist(&mut self, id: &str, cx: &mut gpui::Context<Self>) {
        if self.catalog.read(cx).playlists.value(id).is_none() && external_playlist(id) {
            self.pending_play = Some(id.to_owned());
            self.pending_shuffle = None;
            self.catalog
                .update(cx, |catalog, _| catalog.playlists.remove(id));
            self.ensure_playlist(id, cx);
            self.notify(t("Loading playlist"));
            return;
        }

        if let Some(playlist) = self.playlist(id, cx) {
            self.start_playlist(&playlist, cx);
        }
    }

    pub fn toggle_playlist(&mut self, id: &str, cx: &mut gpui::Context<Self>) {
        if let Some(playlist) = self
            .playlist(id, cx)
            .filter(|playlist| !playlist.songs.is_empty())
        {
            self.toggle_list(&playlist.songs, cx);
            return;
        }

        self.play_playlist(id, cx);
    }

    pub fn toggle_list(&mut self, songs: &[Song], cx: &mut gpui::Context<Self>) {
        if self.player.read(cx).queue.same(songs) {
            self.toggle(cx);
            return;
        }

        self.play_list(songs.to_vec(), 0, cx);
    }

    pub fn play_album(&mut self, id: &str, cx: &mut gpui::Context<Self>) {
        if let Some(album) = self.catalog.read(cx).albums.value(id).cloned() {
            if !album.tracks.is_empty() {
                self.play_list(album.tracks, 0, cx);
            }

            return;
        }

        self.pending_album = Some(id.to_owned());
        self.catalog
            .update(cx, |catalog, _| catalog.albums.remove(id));
        self.ensure_album(id, cx);
        self.notify(t("Loading"));
    }

    pub fn shuffle_playlist(&mut self, id: &str, cx: &mut gpui::Context<Self>) {
        if self.catalog.read(cx).playlists.value(id).is_none() && external_playlist(id) {
            self.pending_shuffle = Some(id.to_owned());
            self.pending_play = None;
            self.catalog
                .update(cx, |catalog, _| catalog.playlists.remove(id));
            self.ensure_playlist(id, cx);
            self.notify(t("Loading playlist"));
            return;
        }

        let Some(playlist) = self.playlist(id, cx) else {
            return;
        };

        self.start_shuffle(&playlist, cx);
    }

    pub(crate) fn start_shuffle(&mut self, playlist: &Playlist, cx: &mut gpui::Context<Self>) {
        if playlist.songs.is_empty() {
            self.fail(format!("{} has no songs", playlist.name));
            return;
        }

        let start = rand::random_range(0..playlist.songs.len());
        self.player.update(cx, |player, _| {
            player.queue.shuffle = true;
            player.queue.set(playlist.songs.clone(), start);
            player.stop();
            player.duration = player
                .current()
                .map(|song| song.duration as f64)
                .unwrap_or(0.0);
            player.progress = 0.0;
        });
        self.play_current(cx);
    }

    pub(crate) fn start_playlist(&mut self, playlist: &Playlist, cx: &mut gpui::Context<Self>) {
        if playlist.songs.is_empty() {
            self.fail(format!("{} has no songs", playlist.name));
            return;
        }

        self.player.update(cx, |player, _| {
            player.queue.set(playlist.songs.clone(), 0);
            player.stop();
            player.duration = player
                .current()
                .map(|song| song.duration as f64)
                .unwrap_or(0.0);
            player.progress = 0.0;
        });
        self.play_current(cx);
    }

    pub fn play_list(&mut self, songs: Vec<Song>, index: usize, cx: &mut gpui::Context<Self>) {
        if songs.is_empty() {
            return;
        }

        // A row of the list already in the queue restarts that track instead of
        // rebuilding the queue: with shuffle on, rebuilding would permute a
        // list that is already playing.
        let clicked = songs.get(index);
        let same = {
            let player = self.player.read(cx);
            player.queue.same(&songs)
                && clicked.is_some_and(|song| {
                    player
                        .current()
                        .is_some_and(|current| current.id == song.id)
                })
        };
        if same {
            self.play_current_at(0.0, cx);
            return;
        }

        self.player.update(cx, |player, _| {
            player.queue.set(songs, index);
            player.stop();
            player.duration = player
                .current()
                .map(|song| song.duration as f64)
                .unwrap_or(0.0);
            player.progress = 0.0;
        });
        self.play_current(cx);
    }

    pub fn play_now(&mut self, song: Song, cx: &mut gpui::Context<Self>) {
        let duration = song.duration as f64;
        let replaced = self
            .player
            .update(cx, |player, _| player.queue.replace_current(song.clone()));
        if !replaced {
            self.play_list(vec![song], 0, cx);
            return;
        }

        self.player.update(cx, |player, _| {
            player.stop();
            player.duration = duration;
            player.progress = 0.0;
        });
        self.persist(cx);
        self.play_current(cx);
    }

    pub fn play_song(&mut self, playlist: &str, index: usize, cx: &mut gpui::Context<Self>) {
        let songs = self.songs(playlist, cx);
        self.play_list(songs, index, cx);
    }

    pub fn identify(&mut self, song: &Song) {
        if song.source != Source::Spotify || song.video().is_some_and(|id| !id.is_empty()) {
            return;
        }

        if links::read(&song.id).is_some() || !self.probes.insert(song.id.clone()) {
            return;
        }

        let youtube = self.youtube.clone();
        let id = song.id.clone();
        let song = song.clone();
        self.jobs.spawn(async move {
            let video = candidates(&youtube, &song)
                .await
                .ok()
                .and_then(|videos| videos.first().map(|video| video.id.clone()));
            Event::YouTubeLink { id, video }
        });
    }

    /// The tracks in line, as the queue would play them.
    fn ahead(&self, cx: &gpui::App, count: usize) -> Vec<Song> {
        let player = self.player.read(cx);
        player
            .queue
            .ahead(player.repeat, count)
            .into_iter()
            .cloned()
            .collect()
    }

    /// Warms the streams of the tracks in line, so skipping does not wait for
    /// the player call. A song whose video is not linked yet is left to
    /// `identify`: its link arrives as an event and warms the stream then. Best
    /// effort, a failed prefetch costs nothing.
    pub(crate) fn prefetch(&self, cx: &gpui::App) {
        if self.rooms.read(cx).live() {
            return;
        }

        let mut videos: Vec<String> = Vec::new();
        for song in self.ahead(cx, AHEAD) {
            let video = song
                .video()
                .filter(|id| !id.is_empty())
                .map(str::to_owned)
                .or_else(|| links::read(&song.id));
            if let Some(video) = video
                && !videos.contains(&video)
                && self.youtube.cached(&video).is_none()
            {
                videos.push(video);
            }
        }

        if videos.is_empty() {
            return;
        }

        let youtube = self.youtube.clone();
        self.jobs.run(async move {
            // Only the URLs are warmed, never the audio: a prefetched stream is
            // two short strings in `Youtube::streams`. The bound is on the
            // player answers in flight while they are parsed.
            futures_util::stream::iter(videos)
                .map(|video| {
                    let youtube = youtube.clone();
                    async move {
                        if let Err(error) = youtube.stream(&video).await {
                            log::debug!("prefetch {video}: {error}");
                        }
                    }
                })
                .buffer_unordered(PARALLEL)
                .for_each(|()| async {})
                .await;
        });
    }

    pub fn play_current(&mut self, cx: &mut gpui::Context<Self>) {
        if self.rooms.read(cx).live() {
            self.start_round(cx);
        } else {
            self.play_current_at(0.0, cx);
        }
    }

    pub fn play_current_at(&mut self, start: f64, cx: &mut gpui::Context<Self>) {
        self.load_current(start, true, false, cx);
    }

    pub fn load_current_at(&mut self, start: f64, play: bool, cx: &mut gpui::Context<Self>) {
        self.load_current(start, play, false, cx);
    }

    /// A stream died mid-track. The same song is resolved again with fresh URLs
    /// and untried candidates once; skipping is only for a load that fails on
    /// its own, which surfaces the error instead of quietly moving on.
    pub(crate) fn recover(&mut self, error: String, cx: &mut gpui::Context<Self>) {
        // The URL that just died is the cached one, and it would die the same
        // way on the next play: drop it before asking for a fresh one.
        let video = self
            .player
            .read(cx)
            .current()
            .and_then(|song| song.video().map(str::to_owned));
        if let Some(video) = video {
            self.youtube.forget(&video);
        }

        if !self.player.update(cx, |player, _| player.retry()) {
            self.player
                .update(cx, |player, _| player.fail(error.clone()));
            self.fail(error);
            return;
        }

        // A room replays the round on the shared clock: retrying the stream
        // alone would put this member's position out of step with the rest.
        if self.rooms.read(cx).live() {
            self.play_current(cx);
            return;
        }

        let (play, start) = {
            let player = self.player.read(cx);
            (player.playing, player.progress)
        };
        self.load_current(start, play, true, cx);
    }

    fn load_current(&mut self, start: f64, play: bool, fresh: bool, cx: &mut gpui::Context<Self>) {
        let Some(song) = self.player.read(cx).current().cloned() else {
            return;
        };

        self.player.update(cx, |player, _| {
            player.stop();
            if !fresh {
                player.retried = false;
            }
            player.loading = true;
            player.error = None;
            player.progress = start.max(0.0);
            player.duration = song.duration as f64;
        });
        self.play_token += 1;
        let token = self.play_token;

        let youtube = self.youtube.clone();
        let known = links::read(&song.id);
        self.jobs.spawn(async move {
            let (resolved, result) = resolve(youtube, song, known, fresh).await;
            Event::YouTubeStream {
                token,
                song: resolved,
                start,
                play,
                result,
            }
        });

        for song in self.ahead(cx, AHEAD) {
            self.identify(&song);
        }
        self.prefetch(cx);
    }

    pub fn toggle(&mut self, cx: &mut gpui::Context<Self>) {
        if self.player.update(cx, |player, _| player.toggle()) {
            let playing = self.player.read(cx).playing;
            let action = if playing { "play" } else { "pause" };
            self.control(cx, serde_json::json!({ "action": action }));
            self.persist(cx);
            // A media key reaches this without a click to repaint the button.
            cx.notify();
            return;
        }

        let (duration, progress) = {
            let player = self.player.read(cx);
            (player.duration, player.progress)
        };
        let has_song = self.player.read(cx).current().is_some();
        if has_song {
            let start = if duration > 0.0 && progress >= duration - 0.2 {
                0.0
            } else {
                progress
            };

            if self.rooms.read(cx).live() {
                self.start_round(cx);
            } else {
                self.play_current_at(start, cx);
            }
        }
    }

    /// The OS stop control ends the track instead of pausing it: the card goes
    /// away and the next play starts from the top.
    pub fn stop(&mut self, cx: &mut gpui::Context<Self>) {
        self.player.update(cx, |player, _| {
            player.stop();
            player.progress = 0.0;
        });
        self.control(cx, serde_json::json!({ "action": "stop" }));
        self.persist(cx);
        cx.notify();
    }

    pub fn next(&mut self, cx: &mut gpui::Context<Self>) {
        let index = {
            let player = self.player.read(cx);
            player.queue.next_index(player.repeat)
        };
        if let Some(index) = index {
            self.player.update(cx, |player, _| player.queue.at = index);
            if self.rooms.read(cx).live() {
                self.start_round(cx);
                return;
            }

            self.play_current_at(0.0, cx);
            self.persist(cx);
        }
    }

    pub fn previous(&mut self, cx: &mut gpui::Context<Self>) {
        if self.player.read(cx).progress > 3.0 {
            self.seek(0.0, cx);
            return;
        }

        let index = {
            let player = self.player.read(cx);
            player.queue.previous_index(player.repeat)
        };
        if let Some(index) = index {
            self.player.update(cx, |player, _| player.queue.at = index);
            if self.rooms.read(cx).live() {
                self.start_round(cx);
            } else {
                self.play_current_at(0.0, cx);
            }

            self.persist(cx);
        }
    }

    pub fn seek(&mut self, seconds: f64, cx: &mut gpui::Context<Self>) {
        self.player.update(cx, |player, _| player.seek(seconds));
        self.control(
            cx,
            serde_json::json!({
                "action": "seek",
                "positionMs": (seconds.max(0.0) * 1000.0).round() as i64,
            }),
        );
    }

    pub fn toggle_shuffle(&mut self, cx: &mut gpui::Context<Self>) {
        let shuffle = self.player.read(cx).queue.shuffle;
        if shuffle {
            self.player
                .update(cx, |player, _| player.queue.set_shuffle(false, None));
            self.control(cx, serde_json::json!({ "action": "shuffle", "on": false }));
        } else {
            let seed = rand::random::<u32>() as u64;
            self.player.update(cx, |player, _| {
                player.queue.set_shuffle(true, Some(seed));
            });
            self.control(
                cx,
                serde_json::json!({
                    "action": "shuffle",
                    "on": true,
                    "seed": seed,
                }),
            );
        }

        self.persist(cx);
    }

    pub fn cycle_repeat(&mut self, cx: &mut gpui::Context<Self>) {
        let repeat = self.player.update(cx, |player, _| {
            player.repeat = player.repeat.next();
            player.repeat
        });
        self.control(
            cx,
            serde_json::json!({
                "action": "repeat",
                "mode": repeat_name(repeat),
            }),
        );
        self.persist(cx);
    }

    pub fn set_volume(&mut self, volume: f32, cx: &mut gpui::Context<Self>) {
        self.player
            .update(cx, |player, _| player.set_volume(volume, false));
    }

    pub fn toggle_mute(&mut self, cx: &mut gpui::Context<Self>) {
        let (muted, volume) = {
            let player = self.player.read(cx);
            (player.muted, player.volume)
        };
        let silent = muted || volume <= 0.001;
        let volume = if volume <= 0.001 { 0.7 } else { volume };

        self.player
            .update(cx, |player, _| player.set_volume(volume, !silent));
        self.persist(cx);
    }
}

use super::resolve::*;
