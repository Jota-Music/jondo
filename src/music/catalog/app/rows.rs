use std::rc::Rc;

use crate::shared::music::domain::Playlist;
use crate::shell::app::{Cache, Jondo, Order, Table};

impl Jondo {
    pub fn album_rows(&mut self, id: &str, cx: &gpui::App) -> Table {
        let key = format!("album|{id}|{}", self.ui.library_revision);
        if self.ui.table.key.as_deref() != Some(key.as_str()) {
            let tracks = self
                .catalog
                .read(cx)
                .albums
                .value(id)
                .map(|album| album.tracks.clone())
                .unwrap_or_default();
            let table = Table::build(tracks, "", Order::Added, false);
            self.ui.table = Table {
                key: Some(key),
                ..table
            };
        }

        self.ui.table.clone()
    }

    pub fn artist_rows(&mut self, id: &str, cx: &gpui::App) -> Table {
        let key = format!("artist|{id}|{}", self.ui.library_revision);
        if self.ui.table.key.as_deref() != Some(key.as_str()) {
            let tracks = self
                .catalog
                .read(cx)
                .artists
                .value(id)
                .map(|artist| artist.tracks.clone())
                .unwrap_or_default();
            let table = Table::build(tracks, "", Order::Added, false);
            self.ui.table = Table {
                key: Some(key),
                ..table
            };
        }

        self.ui.table.clone()
    }

    pub fn user_playlists(&mut self, username: &str, cx: &gpui::App) -> Rc<Vec<Playlist>> {
        let key = format!("user|{username}|{}", self.ui.library_revision);
        if self.ui.user_playlists.key.as_deref() != Some(key.as_str()) {
            let playlists = self
                .catalog
                .read(cx)
                .users
                .value(username)
                .map(|user| user.playlists.clone())
                .unwrap_or_default();
            self.ui.user_playlists = Cache {
                key: Some(key),
                value: Rc::new(playlists),
            };
        }

        self.ui.user_playlists.value.clone()
    }

    pub fn channel_rows(&mut self, id: &str, cx: &gpui::App) -> Table {
        let key = format!(
            "channel|{id}|{}|{:?}|{}|{}",
            self.ui.channel_query,
            self.ui.channel_order,
            self.catalog
                .read(cx)
                .channels
                .value(id)
                .map_or(0, |channel| channel.videos.len()),
            self.ui.library_revision
        );
        if self.ui.table.key.as_deref() != Some(key.as_str()) {
            let videos = self
                .catalog
                .read(cx)
                .channels
                .value(id)
                .map(|channel| channel.videos.clone())
                .unwrap_or_default();
            let table = Table::build(videos, &self.ui.channel_query, self.ui.channel_order, true);
            self.ui.table = Table {
                key: Some(key),
                ..table
            };
        }

        self.ui.table.clone()
    }

    pub fn search_rows(&mut self) -> Table {
        let key = format!(
            "search|{}|{}",
            self.search.query,
            self.search.songs.list().len()
        );
        let Self { ui, search, .. } = self;
        if ui.table.key.as_deref() != Some(key.as_str()) {
            ui.table = Table {
                key: Some(key),
                ..Table::build(search.songs.list().to_vec(), "", Order::Added, false)
            };
        }

        ui.table.clone()
    }

    pub fn catalog_rows(&mut self, cx: &gpui::App) -> Table {
        let catalog = self.catalog.read(cx);
        let ui = &mut self.ui;
        let count = catalog
            .search
            .value
            .as_ref()
            .map_or(0, |search| search.tracks.len());
        let key = format!("catalog|{}|{count}", catalog.query);
        if ui.table.key.as_deref() != Some(key.as_str()) {
            let tracks = catalog
                .search
                .value
                .as_ref()
                .map(|search| search.tracks.clone())
                .unwrap_or_default();
            ui.table = Table {
                key: Some(key),
                ..Table::build(tracks, "", Order::Added, false)
            };
        }

        ui.table.clone()
    }
}
