use std::time::Instant;

use crate::shared::i18n::t;
use crate::shared::music::domain::link::{
    YoutubeRef, direct_url, spotify_id, spotify_link, youtube_ref,
};
use crate::shared::spotify;
use crate::shell::app::{Event, Jondo, Route, SearchState, SearchType, SpotifyState};

impl Jondo {
    pub fn ensure_catalog(&mut self, query: &str, cx: &mut gpui::Context<Self>) {
        {
            let catalog = self.catalog.read(cx);
            if catalog.failed.as_deref() == Some(query) {
                return;
            }

            if catalog.query == query && (catalog.search.value.is_some() || catalog.search.loading)
            {
                return;
            }
        }

        let Some(session) = self.settings.read(cx).auth.clone() else {
            if self.store.spotify.is_some() {
                self.await_catalog(query, cx);
            }

            return;
        };

        self.catalog.update(cx, |catalog, _| {
            catalog.pending = None;
            catalog.begin(query);
        });
        let http = self.http.clone();
        let query = query.to_owned();
        self.jobs.spawn(async move {
            let result = spotify::search(&http, &session, &query).await;
            Event::SpotifyCatalog(query, result)
        });
    }

    /// Holds a query until the stored session finishes connecting. Without
    /// this a search typed during startup fell through to YouTube, which also
    /// rewrote the chosen source and left every later search on YouTube.
    fn await_catalog(&mut self, query: &str, cx: &mut gpui::Context<Self>) {
        self.catalog.update(cx, |catalog, _| {
            catalog.pending = Some(query.to_owned());
            catalog.begin(query);
        });
    }

    fn open_catalog(&mut self, query: &str) {
        self.ui.query = query.to_owned();
        self.history
            .open(Route::Catalog(SearchType::Track, query.to_owned()));
        self.route = self.history.current().clone();
        self.route_changed_at = Some(Instant::now());
        self.clear_selection();
    }

    pub fn catalog_search(&mut self, query: String, cx: &mut gpui::Context<Self>) {
        let query = query.trim().to_owned();
        if query.is_empty() {
            return;
        }

        if let Some((kind, id)) = spotify_link(&query) {
            self.ui.search_open = false;
            self.open_spotify(kind, id, cx);
            return;
        }

        if self.ui.search_type != SearchType::User && spotify_id(&query) {
            let id = query.clone();
            match self.ui.search_type {
                SearchType::Track => self.catalog_track(id, cx),
                SearchType::Album => self.open(Route::Album(id), cx),
                SearchType::Artist => self.open(Route::Artist(id), cx),
                SearchType::Playlist => {
                    self.open(Route::Playlist(format!("spotify:playlist:{id}")), cx)
                }
                SearchType::User => {}
            }

            return;
        }

        if self.ui.search_type == SearchType::User {
            self.open(Route::User(query), cx);
            return;
        }

        if self.ui.search_type != SearchType::Track {
            self.open(Route::Catalog(self.ui.search_type, query), cx);
            return;
        }

        self.catalog_track(query, cx);
    }

    pub(super) fn open_spotify(&mut self, kind: &str, id: String, cx: &mut gpui::Context<Self>) {
        match kind {
            "track" => self.catalog_track(id, cx),
            "user" => self.open(Route::User(id), cx),
            "album" => self.open(Route::Album(id), cx),
            "artist" => self.open(Route::Artist(id), cx),
            "playlist" => self.open(Route::Playlist(format!("spotify:playlist:{id}")), cx),
            _ => {}
        }
    }

    pub(super) fn catalog_track(&mut self, query: String, cx: &mut gpui::Context<Self>) {
        let Some(session) = self.settings.read(cx).auth.clone() else {
            if self.store.spotify.is_none() {
                self.fail(t("Connect Spotify to search its catalog"));
                return;
            }

            self.open_catalog(&query);
            if self.settings.read(cx).spotify == SpotifyState::Failed {
                let error = self
                    .settings
                    .read(cx)
                    .error
                    .clone()
                    .unwrap_or_else(|| t("Spotify is unavailable").to_owned());
                self.catalog.update(cx, |catalog, _| {
                    catalog.search.loading = false;
                    catalog.search.error = Some(error);
                });
            } else {
                self.await_catalog(&query, cx);
                self.notify(t("Connecting to Spotify…"));
            }

            return;
        };

        self.open_catalog(&query);
        self.catalog.update(cx, |catalog, _| {
            catalog.pending = None;
            catalog.begin(&query);
        });
        let http = self.http.clone();
        self.jobs.spawn(async move {
            let result = spotify::search(&http, &session, &query).await;
            Event::SpotifyCatalog(query, result)
        });
    }

    pub fn search(&mut self, query: String, cx: &mut gpui::Context<Self>) {
        let query = query.trim().to_owned();
        if query.is_empty() {
            return;
        }

        self.clear_selection();
        self.ui.query = query.clone();
        self.ui.search_closing = None;
        let reference = youtube_ref(&query);
        if let Some(YoutubeRef::Channel(id)) = &reference {
            self.ui.search_open = false;
            self.open(Route::Channel(id.clone()), cx);
            return;
        }

        if let Some((kind, id)) = spotify_link(&query) {
            self.ui.search_open = false;
            self.open_spotify(kind, id, cx);
            return;
        }

        let direct = reference.is_some() || direct_url(&query);
        let playlists = !direct || matches!(&reference, Some(YoutubeRef::Playlist(_)));
        let videos = !direct || !playlists;
        let channels = !direct;
        let tab = if matches!(&reference, Some(YoutubeRef::Playlist(_))) {
            "playlists"
        } else {
            "videos"
        };

        self.ui.search_open = true;
        self.history.open(Route::Search(query.clone()));
        self.route = self.history.current().clone();
        self.route_key = format!("route-{:?}", self.route).into();
        self.route_changed_at = Some(Instant::now());
        let list = self.search.list;
        let mut search = SearchState {
            query: query.clone(),
            scroll_top: true,
            tab: tab.to_owned(),
            list,
            ..Default::default()
        };
        search.songs.loading = videos;
        search.channels.loading = channels;
        search.playlists.loading = playlists;
        self.search = search;

        let youtube = self.youtube.clone();
        if videos {
            let videos = youtube.clone();
            let video_query = query.clone();
            self.jobs.spawn(async move {
                let result = videos.search(&video_query).await;
                Event::YouTubeSearch(video_query, result)
            });
        }

        if channels {
            let channels = youtube.clone();
            let channel_query = query.clone();
            self.jobs.spawn(async move {
                let result = channels.search_channels(&channel_query).await;
                Event::YouTubeChannels(channel_query, result)
            });
        }

        if playlists {
            let playlists = youtube;
            let playlist_query = query.clone();
            self.jobs.spawn(async move {
                let result = playlists.search_playlists(&playlist_query).await;
                Event::YouTubePlaylists(playlist_query, result)
            });
        }
    }
}
