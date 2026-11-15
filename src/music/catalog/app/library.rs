use crate::shared::music::domain::{Album, ArtistView, ChannelView, UserView};
use crate::shared::refresh::stale;
use crate::shared::spotify;
use crate::shell::app::{Event, Jondo};

impl Jondo {
    pub fn ensure_discography(
        &mut self,
        artist_id: &str,
        group: &str,
        cx: &mut gpui::Context<Self>,
    ) {
        let key = (artist_id.to_owned(), group.to_owned());
        {
            let catalog = self.catalog.read(cx);
            if catalog.discography.contains(&key) || catalog.discography_loading.contains(&key) {
                return;
            }
        }

        let albums: Vec<Album> = {
            let catalog = self.catalog.read(cx);
            let Some(artist) = catalog.artists.value(artist_id) else {
                return;
            };

            artist
                .albums
                .iter()
                .filter(|album| album.group == group)
                .cloned()
                .collect()
        };

        if albums.is_empty() {
            self.catalog
                .update(cx, |catalog, _| catalog.discography.insert(key));
            return;
        }

        if albums
            .iter()
            .all(|album| album.cover.is_some() && !album.name.is_empty())
        {
            self.catalog
                .update(cx, |catalog, _| catalog.discography.insert(key));
            return;
        }

        let Some(session) = self.settings.read(cx).auth.clone() else {
            return;
        };

        let http = self.http.clone();
        let artist_id = artist_id.to_owned();
        let group = group.to_owned();
        self.catalog
            .update(cx, |catalog, _| catalog.discography_loading.insert(key));
        self.jobs.spawn(async move {
            let mut albums = albums;
            spotify::enrich_albums(&http, &session, &mut albums).await;
            Event::SpotifyDiscography(artist_id, group, Ok(albums))
        });
    }

    pub fn discography_ready(&self, artist_id: &str, group: &str, cx: &gpui::App) -> bool {
        self.catalog
            .read(cx)
            .discography
            .contains(&(artist_id.to_owned(), group.to_owned()))
    }

    pub fn ensure_album(&mut self, id: &str, cx: &mut gpui::Context<Self>) {
        if self.catalog.read(cx).albums.waiting(id) {
            return;
        }

        if let Some((album, saved)) = spotify::cached_album(id) {
            self.catalog
                .update(cx, |catalog, _| catalog.albums.ready(id.to_owned(), album));
            if !stale(saved) {
                return;
            }
        }

        self.load_album(id, cx);
    }

    /// Fetches an album again over the copy already on screen, so reopening a
    /// disc refreshes it in place instead of blanking the page first.
    pub fn reload_album(&mut self, id: &str, cx: &mut gpui::Context<Self>) {
        if self.catalog.read(cx).albums.loading(id) {
            return;
        }

        self.load_album(id, cx);
    }

    fn load_album(&mut self, id: &str, cx: &mut gpui::Context<Self>) {
        let Some(session) = self.settings.read(cx).auth.clone() else {
            return;
        };

        if !self
            .catalog
            .update(cx, |catalog, _| catalog.albums.fetch(id))
        {
            return;
        }

        let id = id.to_owned();
        let http = self.http.clone();
        self.jobs.spawn(async move {
            let result = spotify::album(&http, &session, &id).await;
            Event::SpotifyAlbum(id, result)
        });
    }

    pub fn ensure_artist(&mut self, id: &str, cx: &mut gpui::Context<Self>) {
        if self.catalog.read(cx).artists.waiting(id) {
            return;
        }

        if let Some((artist, saved)) = spotify::cached_artist(id) {
            self.catalog.update(cx, |catalog, _| {
                catalog.artists.ready(id.to_owned(), artist)
            });
            if !stale(saved) {
                return;
            }
        }

        self.load_artist(id, cx);
    }

    /// Fetches an artist again over the copy already on screen.
    pub fn reload_artist(&mut self, id: &str, cx: &mut gpui::Context<Self>) {
        if self.catalog.read(cx).artists.loading(id) {
            return;
        }

        self.load_artist(id, cx);
    }

    fn load_artist(&mut self, id: &str, cx: &mut gpui::Context<Self>) {
        let Some(session) = self.settings.read(cx).auth.clone() else {
            return;
        };

        if !self
            .catalog
            .update(cx, |catalog, _| catalog.artists.fetch(id))
        {
            return;
        }

        let id = id.to_owned();
        let http = self.http.clone();
        self.jobs.spawn(async move {
            let result = spotify::artist(&http, &session, &id).await;
            Event::SpotifyArtist(id, result)
        });
    }

    /// Loads the missing album covers for every group the artist page shows.
    /// Called when the artist arrives, not while the page renders.
    pub fn ensure_artist_meta(&mut self, id: &str, cx: &mut gpui::Context<Self>) {
        let groups = {
            let catalog = self.catalog.read(cx);
            let Some(artist) = catalog.artists.value(id) else {
                return;
            };

            crate::shared::music::domain::groups(&artist.albums)
        };

        for group in groups {
            self.ensure_discography(id, &group, cx);
        }
    }

    pub fn ensure_channel(&mut self, id: &str, cx: &mut gpui::Context<Self>) {
        if self.catalog.read(cx).channels.waiting(id) {
            return;
        }

        if let Some((channel, saved)) = crate::shared::youtube::cached_channel(id) {
            self.catalog.update(cx, |catalog, _| {
                catalog.channels.ready(id.to_owned(), channel)
            });
            if !stale(saved) {
                return;
            }
        }

        self.load_channel(id, cx);
    }

    /// Fetches a channel again over the copy already on screen.
    pub fn reload_channel(&mut self, id: &str, cx: &mut gpui::Context<Self>) {
        if self.catalog.read(cx).channels.loading(id) {
            return;
        }

        self.load_channel(id, cx);
    }

    fn load_channel(&mut self, id: &str, cx: &mut gpui::Context<Self>) {
        if !self
            .catalog
            .update(cx, |catalog, _| catalog.channels.fetch(id))
        {
            return;
        }

        let id = id.to_owned();
        let youtube = self.youtube.clone();
        self.jobs.spawn(async move {
            let result = youtube.channel(&id).await;
            Event::YouTubeChannel(id, result)
        });
    }

    pub fn channel(&self, id: &str, cx: &gpui::App) -> Option<ChannelView> {
        self.catalog.read(cx).channels.value(id).cloned()
    }

    pub fn ensure_user(&mut self, username: &str, cx: &mut gpui::Context<Self>) {
        if self.catalog.read(cx).users.waiting(username) {
            return;
        }

        if let Some((user, saved)) = spotify::cached_user(username) {
            self.catalog.update(cx, |catalog, _| {
                catalog.users.ready(username.to_owned(), user)
            });
            if !stale(saved) {
                return;
            }
        }

        self.load_user(username, cx);
    }

    /// Fetches a profile again over the copy already on screen.
    pub fn reload_user(&mut self, username: &str, cx: &mut gpui::Context<Self>) {
        if self.catalog.read(cx).users.loading(username) {
            return;
        }

        self.load_user(username, cx);
    }

    fn load_user(&mut self, username: &str, cx: &mut gpui::Context<Self>) {
        let Some(session) = self.settings.read(cx).auth.clone() else {
            return;
        };

        if !self
            .catalog
            .update(cx, |catalog, _| catalog.users.fetch(username))
        {
            return;
        }

        let username = username.to_owned();
        let http = self.http.clone();
        self.jobs.spawn(async move {
            let result = spotify::user(&http, &session, &username).await;
            Event::SpotifyUser(username, result)
        });
    }

    pub fn user(&self, username: &str, cx: &gpui::App) -> Option<UserView> {
        self.catalog.read(cx).users.value(username).cloned()
    }

    pub fn album(&self, id: &str, cx: &gpui::App) -> Option<Album> {
        self.catalog.read(cx).albums.value(id).cloned()
    }

    pub fn artist(&self, id: &str, cx: &gpui::App) -> Option<ArtistView> {
        self.catalog.read(cx).artists.value(id).cloned()
    }
}
