use crate::shared::i18n::t;

use super::*;

impl Jondo {
    pub(super) fn youtube_event(&mut self, event: Event, cx: &mut gpui::Context<Self>) {
        match event {
            Event::YouTubeLibrary(token, result) => {
                if token != self.settings.read(cx).youtube_load {
                    return;
                }

                match result {
                    Ok(playlists) => {
                        log::info!("youtube: library loaded: {} playlists", playlists.len());
                        self.library
                            .update(cx, |library, _| library.youtube = playlists);
                        self.settings
                            .update(cx, |settings, _| settings.youtube_expired = false);
                        self.touch_library();
                    }

                    Err(error)
                        if error.contains("401 Unauthorized")
                            || error == crate::shared::youtube::EXPIRED =>
                    {
                        log::info!("youtube: session expired: {error}");
                        self.settings
                            .update(cx, |settings, _| settings.youtube_expired = true);
                        if !self.store.youtube_browser.is_empty() {
                            self.refresh_browser(true, cx);
                        } else if !self.settings.read(cx).youtube_rescue {
                            self.notify(t("The YouTube session expired. Import it again."));
                        }
                    }
                    Err(error) => log::warn!("youtube: library: {error}"),
                }
            }
            Event::YouTubeSearch(query, result) => {
                if self.search.query == query {
                    let result = result.map(|videos| {
                        videos
                            .into_iter()
                            .map(|video| video.song())
                            .collect::<Vec<_>>()
                    });
                    let ok = result.is_ok();
                    self.search.songs.settle(result);
                    if ok {
                        self.search.scroll_top = true;
                    }
                }
            }
            Event::YouTubeVideos(query, result) => {
                if self.editor.open && self.editor.query == query {
                    self.editor.results.settle(result);
                }
            }
            Event::YouTubeLink { id, video } => {
                if let Some(video) = video
                    && self.store.link(&id, &video)
                {
                    self.player.update(cx, |player, _| {
                        if let Some(current) = player.queue.current_mut()
                            && current.id == id
                        {
                            current.video_id = Some(video);
                        }
                    });

                    self.persist(cx);
                }
            }
            Event::YouTubePreview { token, result } => {
                let preview = &mut self.preview;
                let loaded = self
                    .player
                    .update(cx, |player, _| preview.load(player, token, result));
                if let Err(error) = loaded {
                    let preview = &mut self.preview;
                    self.player.update(cx, |player, _| preview.stop(player));
                    self.fail(t("Could not play this video."));
                    log::warn!("youtube preview: {error}");
                }
            }
            Event::YouTubeChannels(query, result) => {
                if self.search.query == query {
                    self.search.channels.settle(result);
                }
            }
            Event::YouTubePlaylists(query, result) => {
                if self.search.query == query {
                    self.search.playlists.settle(result);
                }
            }
            Event::YouTubePlaylist(id, result) => match result {
                Ok(playlist) => {
                    if self.sync_saved(&playlist) {
                        self.persist(cx);
                    }

                    let cached = playlist.clone();
                    self.jobs.task(move || cache::put(&cached));
                    self.remember(&id, playlist.clone(), cx);
                    self.library.update(cx, |library, _| {
                        if let Some(summary) = library.youtube.iter_mut().find(|item| item.id == id)
                        {
                            summary.track_count = playlist.track_count;
                            if summary.cover.is_none() {
                                summary.cover = playlist.cover.clone();
                            }
                        }
                    });

                    if self.pending_play.as_deref() == Some(&id) {
                        self.pending_play = None;
                        self.start_playlist(&playlist, cx);
                    } else if self.pending_shuffle.as_deref() == Some(&id) {
                        self.pending_shuffle = None;
                        self.start_shuffle(&playlist, cx);
                    }
                }
                Err(error) => {
                    log::warn!("youtube playlist {id}: {error}");
                    self.catalog
                        .update(cx, |catalog, _| catalog.playlists.fail(id.clone(), error));
                    if self.pending_play.as_deref() == Some(&id) {
                        self.pending_play = None;
                    }

                    if self.pending_shuffle.as_deref() == Some(&id) {
                        self.pending_shuffle = None;
                    }
                }
            },
            Event::YouTubeChannel(id, result) => match result {
                Ok(channel) => {
                    self.catalog
                        .update(cx, |catalog, _| catalog.channels.ready(id, channel));
                    self.touch_library();
                }
                Err(error) => {
                    log::warn!("youtube channel {id}: {error}");
                    self.catalog
                        .update(cx, |catalog, _| catalog.channels.fail(id, error));
                }
            },

            Event::YouTubeStream {
                token,
                song,
                start,
                play,
                result,
            } => {
                if token != self.play_token {
                    return;
                }

                match result {
                    Ok(stream) => {
                        let linked = song.video().unwrap_or_default().to_owned();
                        self.player.update(cx, |player, _| {
                            if let Some(current) = player.queue.current_mut() {
                                current.video_id = song.video_id.clone();
                            }
                        });

                        self.store.link(&song.id, &linked);
                        self.player.update(cx, |player, _| {
                            if play {
                                player.start(stream, start);
                            } else {
                                player.prepare(stream, start);
                            }
                        });

                        self.prefetch(cx);
                        self.persist(cx);
                    }
                    Err(error) => {
                        self.player
                            .update(cx, |player, _| player.fail(error.clone()));
                        self.fail(error);
                    }
                }
            }
            _ => {}
        }
    }
}
