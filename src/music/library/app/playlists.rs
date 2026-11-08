use crate::shared::music::domain::Playlist;
use crate::shared::spotify;

use crate::shell::app::IN_MEMORY_PLAYLISTS;
use crate::shell::app::{Event, Jondo, cache};

impl Jondo {
    pub fn ensure_playlist(&mut self, id: &str, cx: &mut gpui::Context<Self>) {
        if self.catalog.read(cx).playlists.waiting(id) {
            return;
        }

        if let Some((playlist, fresh)) = cache::get(id) {
            self.remember(id, playlist, cx);
            if fresh {
                return;
            }
        }

        self.load_playlist(id, cx);
    }

    pub(crate) fn remember(&mut self, id: &str, playlist: Playlist, cx: &mut gpui::Context<Self>) {
        let evicted = self.catalog.update(cx, |catalog, _| {
            catalog.playlists.ready(id.to_owned(), playlist);
            catalog.recent.retain(|item| item != id);
            catalog.recent.push_back(id.to_owned());
            let mut evicted = Vec::new();
            while catalog.recent.len() > IN_MEMORY_PLAYLISTS {
                let Some(oldest) = catalog.recent.pop_front() else {
                    break;
                };

                catalog.playlists.remove(&oldest);
                evicted.push(oldest);
            }
            evicted
        });
        self.touch_library();
        for oldest in evicted {
            self.scrolls.remove(&format!("playlist:{oldest}"));
            self.panes.remove(&format!("playlist:{oldest}"));
        }
    }

    pub fn reload_playlist(&mut self, id: &str, cx: &mut gpui::Context<Self>) {
        if self.catalog.read(cx).playlists.loading(id) {
            return;
        }

        self.catalog
            .update(cx, |catalog, _| catalog.playlists.remove(id));
        self.load_playlist(id, cx);
    }

    pub(super) fn load_playlist(&mut self, id: &str, cx: &mut gpui::Context<Self>) {
        if id.starts_with("local:") || self.catalog.read(cx).playlists.loading(id) {
            return;
        }

        if let Some(youtube_id) = id.strip_prefix("youtube:playlist:") {
            let id = id.to_owned();
            let youtube_id = youtube_id.to_owned();
            let youtube = self.youtube.clone();
            self.catalog
                .update(cx, |catalog, _| catalog.playlists.begin(&id));
            self.jobs.spawn(async move {
                let result = youtube.playlist(&youtube_id).await;
                Event::YouTubePlaylist(id, result)
            });
            return;
        }

        let Some(session) = self.settings.read(cx).auth.clone() else {
            return;
        };

        if id.starts_with("spotify:user:") && id.ends_with(":collection") {
            self.catalog
                .update(cx, |catalog, _| catalog.playlists.begin(id));
            let id = id.to_owned();
            let http = self.http.clone();
            self.jobs.spawn(async move {
                let result = spotify::liked(&http, &session).await;
                Event::SpotifyPlaylist(id, result)
            });
            return;
        }

        let Some(spotify_id) = id.strip_prefix("spotify:playlist:") else {
            return;
        };

        self.catalog
            .update(cx, |catalog, _| catalog.playlists.begin(id));
        let id = id.to_owned();
        let spotify_id = spotify_id.to_owned();
        let http = self.http.clone();
        self.jobs.spawn(async move {
            let result = spotify::playlist(&http, &session, &spotify_id).await;
            Event::SpotifyPlaylist(id, result)
        });
    }
}
