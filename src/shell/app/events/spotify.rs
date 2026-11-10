use crate::shared::i18n::t;

use super::*;

impl Jondo {
    pub(super) fn spotify_event(&mut self, event: Event, cx: &mut gpui::Context<Self>) {
        match event {
            Event::SpotifyLogin(Ok(login)) => {
                self.store.spotify_at = crate::shared::kernel::app::clock::now_ms();
                self.store.spotify = Some(login.auth);
                self.adopt_spotify(login.session, cx);
                self.notify(t("Signed in to Spotify"));
            }
            Event::SpotifyLogin(Err(error)) => {
                self.settings
                    .update(cx, |settings, _| settings.spotify = SpotifyState::Failed);
                self.settings
                    .update(cx, |settings, _| settings.error = Some(error.clone()));
                self.fail(error);
                self.arm_spotify(cx);
            }
            Event::SpotifyResumed(session) => self.adopt_spotify(session, cx),
            Event::SpotifyLibrary(result) => {
                self.library
                    .update(cx, |library, _| library.load.loading = false);
                match result {
                    Ok(playlists) => {
                        log::info!("spotify: library loaded: {} playlists", playlists.len());
                        let cached = playlists.clone();
                        let user = self.settings.read(cx).user.clone();
                        self.jobs.task(move || cache::save_library(&user, &cached));
                        self.library.update(cx, |library, _| {
                            library.spotify = playlists;
                            library.load.error = None;
                        });
                        self.touch_library();
                        if self.store.spotify.is_some() {
                            self.settings
                                .update(cx, |settings, _| settings.spotify = SpotifyState::On);
                            self.settings
                                .update(cx, |settings, _| settings.error = None);
                        }
                    }
                    Err(error) => {
                        self.settings
                            .update(cx, |settings, _| settings.spotify = SpotifyState::Failed);
                        self.settings
                            .update(cx, |settings, _| settings.error = Some(error.clone()));
                        self.fail(error.clone());
                        self.library
                            .update(cx, |library, _| library.load.error = Some(error));
                        self.arm_spotify(cx);
                    }
                }
            }
            Event::SpotifyPlaylist(id, result) => match result {
                Ok(playlist) => {
                    if self.sync_saved(&playlist) {
                        self.persist(cx);
                    }

                    let cached = playlist.clone();
                    // Radio is recomputed on every open, so it never reaches
                    // the on-disk playlist cache.
                    if !id.starts_with("spotify:radio:") {
                        self.jobs.task(move || cache::put(&cached));
                    }
                    let mut changed = false;
                    self.library.update(cx, |library, _| {
                        if let Some(summary) = library.spotify.iter_mut().find(|item| item.id == id)
                        {
                            changed = summary.track_count != playlist.track_count;
                            summary.track_count = playlist.track_count;
                            if summary.cover.is_none() {
                                summary.cover = playlist.cover.clone();
                            }
                        }
                    });

                    if changed {
                        let cached = self.library.read(cx).spotify.clone();
                        let user = self.settings.read(cx).user.clone();
                        self.jobs.task(move || cache::save_library(&user, &cached));
                    }

                    self.remember(&id, playlist.clone(), cx);
                    if self.pending_play.as_deref() == Some(&id) {
                        self.pending_play = None;
                        self.start_playlist(&playlist, cx);
                    } else if self.pending_shuffle.as_deref() == Some(&id) {
                        self.pending_shuffle = None;
                        self.start_shuffle(&playlist, cx);
                    }
                }
                Err(error) => {
                    log::warn!("spotify playlist {id}: {error}");
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
            Event::SpotifyAlbum(id, result) => match result {
                Ok(album) => {
                    self.catalog.update(cx, |catalog, _| {
                        catalog.albums.ready(id.clone(), album.clone())
                    });
                    self.touch_library();
                    if self.pending_album.as_deref() == Some(&id) {
                        self.pending_album = None;
                        if !album.tracks.is_empty() {
                            self.play_list(album.tracks, 0, cx);
                        }
                    }
                }
                Err(error) => {
                    log::warn!("spotify album {id}: {error}");
                    self.catalog
                        .update(cx, |catalog, _| catalog.albums.fail(id, error));
                }
            },
            Event::SpotifyArtist(id, result) => match result {
                Ok(artist) => {
                    self.catalog.update(cx, |catalog, _| {
                        catalog
                            .discography
                            .retain(|(artist_id, _)| artist_id != &id);
                        catalog.artists.ready(id.clone(), artist);
                    });
                    if matches!(&self.route, Route::Artist(current) if current == &id) {
                        self.ensure_artist_meta(&id, cx);
                    }

                    self.touch_library();
                }
                Err(error) => {
                    log::warn!("spotify artist {id}: {error}");
                    self.catalog
                        .update(cx, |catalog, _| catalog.artists.fail(id, error));
                }
            },
            Event::SpotifyDiscography(artist_id, group, result) => {
                self.catalog.update(cx, |catalog, _| {
                    catalog
                        .discography_loading
                        .remove(&(artist_id.clone(), group.clone()));
                    catalog
                        .discography
                        .insert((artist_id.clone(), group.clone()));
                    if let Ok(albums) = result
                        && let Some(artist) = catalog.artists.value_mut(&artist_id)
                    {
                        for album in albums {
                            if let Some(current) =
                                artist.albums.iter_mut().find(|item| item.id == album.id)
                            {
                                *current = album;
                            }
                        }
                    }
                });
                self.touch_library();
            }
            Event::SpotifyFollowing(result) => {
                self.library.update(cx, |library, _| {
                    library.following.loading = false;
                    library.following.value = Some(());
                });
                match result {
                    Ok(follows) => {
                        self.library.update(cx, |library, _| {
                            library.follows = follows;
                            library.following.error = None;
                        });
                        self.ensure_follow_meta(cx);
                        self.touch_library();
                    }
                    Err(error) => {
                        log::warn!("spotify following: {error}");
                        self.library
                            .update(cx, |library, _| library.following.error = Some(error));
                    }
                }
            }
            Event::SpotifyFriends(result) => match result {
                Ok(friends) => {
                    self.library
                        .update(cx, |library, _| library.friends = friends);
                    self.touch_library();
                }
                Err(error) => log::debug!("spotify friends: {error}"),
            },
            Event::SpotifyUser(username, result) => match result {
                Ok(user) => {
                    self.catalog
                        .update(cx, |catalog, _| catalog.users.ready(username, user));
                    self.touch_library();
                }
                Err(error) => {
                    log::warn!("spotify user {username}: {error}");
                    self.catalog
                        .update(cx, |catalog, _| catalog.users.fail(username, error));
                }
            },
            Event::SpotifyCatalog(query, result) => {
                if self.catalog.read(cx).query != query {
                    return;
                }

                self.catalog.update(cx, |catalog, _| match result {
                    Ok(search) => catalog.search.settle(Ok(search)),
                    Err(error) => {
                        catalog.failed = Some(catalog.query.clone());
                        catalog.search.settle(Err(error));
                    }
                });
            }
            Event::SpotifyFailed(error) => {
                self.settings
                    .update(cx, |settings, _| settings.spotify = SpotifyState::Failed);
                self.settings
                    .update(cx, |settings, _| settings.error = Some(error.clone()));
                self.fail(error.clone());
                self.arm_spotify(cx);
                self.catalog.update(cx, |catalog, _| {
                    if catalog.pending.take().is_some() {
                        catalog.search.settle(Err(error));
                    }
                });
            }
            _ => {}
        }
    }
}
