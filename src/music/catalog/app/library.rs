use crate::shared::music::domain::{Album, ArtistView, ChannelView, UserView};
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

        let Some(session) = self.settings.read(cx).auth.clone() else {
            return;
        };

        self.catalog
            .update(cx, |catalog, _| catalog.albums.begin(id));

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

        let Some(session) = self.settings.read(cx).auth.clone() else {
            return;
        };

        self.catalog
            .update(cx, |catalog, _| catalog.artists.begin(id));

        let id = id.to_owned();
        let http = self.http.clone();
        self.jobs.spawn(async move {
            let result = spotify::artist(&http, &session, &id).await;
            Event::SpotifyArtist(id, result)
        });
    }

    pub fn ensure_channel(&mut self, id: &str, cx: &mut gpui::Context<Self>) {
        if !self
            .catalog
            .update(cx, |catalog, _| catalog.channels.begin(id))
        {
            return;
        }

        self.load_channel(id);
    }

    pub(super) fn load_channel(&mut self, id: &str) {
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

    pub fn reload_channel(&mut self, id: &str, cx: &mut gpui::Context<Self>) {
        if self.catalog.read(cx).channels.loading(id) {
            return;
        }

        self.catalog
            .update(cx, |catalog, _| catalog.channels.remove(id));
        self.load_channel(id);
    }

    pub fn ensure_user(&mut self, username: &str, cx: &mut gpui::Context<Self>) {
        if self.catalog.read(cx).users.waiting(username) {
            return;
        }

        let Some(session) = self.settings.read(cx).auth.clone() else {
            return;
        };

        self.catalog
            .update(cx, |catalog, _| catalog.users.begin(username));

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
