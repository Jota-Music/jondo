use std::collections::HashSet;
use std::sync::Arc;

use crate::shared::music::domain::{Follow, Source};
use crate::shared::spotify;
use crate::shell::app::{Event, Jondo, Route};

impl Jondo {
    pub fn toggle_follow(&mut self, follow: &Follow, cx: &mut gpui::Context<Self>) {
        let mut follow = follow.clone();
        let target = follow.target().to_owned();
        if target.is_empty() {
            return;
        }

        follow.id = Follow::key(follow.kind, &target);
        if follow.name.is_empty() {
            follow.name = target;
        }

        if follow.source.is_none() {
            follow.source = Some(match follow.kind {
                crate::shared::music::domain::FollowKind::Channel => Source::YouTube,
                _ => Source::Spotify,
            });
        }

        if let Some(position) = self
            .store
            .follows
            .iter()
            .position(|item| item.id.eq_ignore_ascii_case(&follow.id))
        {
            self.store.follows.remove(position);
            self.store.follows_at.remove(&follow.id);
            self.store.follow_graves.insert(
                follow.id.clone(),
                crate::shared::kernel::app::clock::now_ms(),
            );
        } else {
            self.store.follows_at.insert(
                follow.id.clone(),
                crate::shared::kernel::app::clock::now_ms(),
            );
            self.store.follows.push(follow);
        }

        self.library_changed(cx);
    }

    pub fn following(&self) -> &[Follow] {
        &self.store.follows
    }

    pub fn following_all(&self, cx: &gpui::App) -> Vec<Follow> {
        let mut entries = Vec::new();
        let mut seen = HashSet::new();
        let mut add = |follow: Follow| {
            if follow.id.is_empty() || !seen.insert(follow.id.to_lowercase()) {
                return;
            }

            entries.push(follow);
        };

        for follow in &self.store.follows {
            add(follow.clone());
        }

        {
            let library = self.library.read(cx);
            for follow in &library.follows {
                add(Follow {
                    id: Follow::key(follow.kind, &follow.id),
                    name: follow.name.clone(),
                    image: follow.image.clone(),
                    kind: follow.kind,
                    source: follow.source.or(Some(Source::Spotify)),
                });
            }

            for user in &library.friends {
                add(Follow {
                    id: user.clone(),
                    name: user.clone(),
                    image: None,
                    kind: crate::shared::music::domain::FollowKind::User,
                    source: Some(Source::Spotify),
                });
            }
        }

        let catalog = self.catalog.read(cx);
        for follow in &mut entries {
            let target = follow.target();
            match follow.kind {
                crate::shared::music::domain::FollowKind::Artist => {
                    if let Some(artist) = catalog.artists.value(target) {
                        if !artist.name.is_empty() {
                            follow.name = artist.name.clone();
                        }

                        if follow.image.is_none() {
                            follow.image = artist.image.clone();
                        }
                    }
                }
                crate::shared::music::domain::FollowKind::Channel => {
                    if let Some(channel) = catalog.channels.value(target) {
                        if !channel.name.is_empty() {
                            follow.name = channel.name.clone();
                        }

                        if follow.image.is_none() {
                            follow.image = channel.avatar.clone();
                        }
                    }
                }
                crate::shared::music::domain::FollowKind::User => {
                    if let Some(user) = catalog.users.value(target) {
                        if !user.name.is_empty() {
                            follow.name = user.name.clone();
                        }

                        if follow.image.is_none() {
                            follow.image = user.image.clone();
                        }
                    }
                }
            }
        }

        entries
    }

    pub fn follow_route(&self, follow: &Follow) -> Route {
        match follow.kind {
            crate::shared::music::domain::FollowKind::Artist => {
                Route::Artist(follow.target().to_owned())
            }
            crate::shared::music::domain::FollowKind::Channel => {
                Route::Channel(follow.target().to_owned())
            }
            crate::shared::music::domain::FollowKind::User => {
                Route::User(follow.target().to_owned())
            }
        }
    }

    pub fn ensure_follow_meta(&mut self, cx: &mut gpui::Context<Self>) {
        for follow in self.follows(cx).iter() {
            let target = follow.target().to_owned();
            if target.is_empty() {
                continue;
            }

            let known = !follow.name.is_empty() && follow.name != target;
            if known && follow.image.is_some() {
                continue;
            }

            match follow.kind {
                crate::shared::music::domain::FollowKind::Artist => self.ensure_artist(&target, cx),
                crate::shared::music::domain::FollowKind::Channel => {
                    self.ensure_channel(&target, cx)
                }
                crate::shared::music::domain::FollowKind::User => self.ensure_user(&target, cx),
            }
        }
    }
}

impl Jondo {
    pub fn load_following(&mut self, session: Arc<spotify::Session>, cx: &mut gpui::Context<Self>) {
        if self.library.read(cx).following.loading || self.settings.read(cx).user.is_empty() {
            return;
        }

        self.library
            .update(cx, |library, _| library.following.loading = true);
        let http = self.http.clone();
        let username = self.settings.read(cx).user.clone();
        let following_session = session.clone();
        self.jobs.spawn(async move {
            Event::SpotifyFollowing(spotify::following(&http, &following_session, &username).await)
        });
        let http = self.http.clone();
        self.jobs
            .spawn(async move { Event::SpotifyFriends(spotify::friends(&http, &session).await) });
    }

    pub fn ensure_following(&mut self, cx: &mut gpui::Context<Self>) {
        let library = self.library.read(cx);
        if library.following.value.is_some() || library.following.loading {
            return;
        }

        let Some(session) = self.settings.read(cx).auth.clone() else {
            return;
        };

        self.load_following(session, cx);
    }
}
