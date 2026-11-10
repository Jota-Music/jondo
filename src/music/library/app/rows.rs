use std::collections::HashSet;
use std::rc::Rc;

use crate::shared::music::domain::{Follow, Playlist, Song, Source};
use crate::shell::app::{Cache, Filter, Jondo, Table};

impl Jondo {
    pub fn shelf(&mut self, cx: &mut gpui::Context<Self>) -> Rc<Vec<Playlist>> {
        let key = format!("shelf|{:?}|{}", self.ui.filter, self.ui.library_revision);
        if self.ui.shelf.key.as_deref() != Some(key.as_str()) {
            let items = self.home_playlists(cx);
            self.ui.shelf = Cache {
                key: Some(key),
                value: Rc::new(items),
            };
        }

        self.ui.shelf.value.clone()
    }

    pub fn follows(&mut self, cx: &mut gpui::Context<Self>) -> Rc<Vec<Follow>> {
        let key = format!("follows|{}", self.ui.library_revision);
        if self.ui.follows.key.as_deref() != Some(key.as_str()) {
            let items = self.following_all(cx);
            self.ui.follows = Cache {
                key: Some(key),
                value: Rc::new(items),
            };
        }

        self.ui.follows.value.clone()
    }

    pub fn following_page(&mut self, cx: &mut gpui::Context<Self>) -> Rc<Vec<Follow>> {
        let key = format!(
            "following|{:?}|{}",
            self.ui.following_filter, self.ui.library_revision
        );
        if self.ui.following_page.key.as_deref() != Some(key.as_str()) {
            let items: Vec<Follow> = self
                .follows(cx)
                .iter()
                .filter(|follow| match self.ui.following_filter {
                    Filter::All => true,
                    Filter::YouTube => {
                        follow.kind == crate::shared::music::domain::FollowKind::Channel
                    }
                    Filter::Spotify => {
                        follow.kind != crate::shared::music::domain::FollowKind::Channel
                    }
                    Filter::Local => false,
                })
                .cloned()
                .collect();
            self.ui.following_page = Cache {
                key: Some(key),
                value: Rc::new(items),
            };
        }

        self.ui.following_page.value.clone()
    }

    pub fn view(&self, id: &str, cx: &gpui::App) -> Option<Playlist> {
        if let Some(local) = self.store.playlist(id) {
            return Some(Playlist {
                id: local.id.clone(),
                name: local.name.clone(),
                covers: local.covers(),
                source: Source::Local,
                track_count: local.songs.len() as u32,
                ..Default::default()
            });
        }

        if let Some(playlist) = self.catalog.read(cx).playlists.value(id) {
            return Some(Playlist {
                id: playlist.id.clone(),
                name: playlist.name.clone(),
                subtitle: playlist.subtitle.clone(),
                cover: playlist.cover.clone(),
                covers: playlist.covers.clone(),
                source: playlist.source,
                owner: playlist.owner.clone(),
                owner_id: playlist.owner_id.clone(),
                songs: Vec::new(),
                track_count: if playlist.track_count > 0 {
                    playlist.track_count
                } else {
                    playlist.songs.len() as u32
                },
                loading: playlist.loading,
            });
        }

        let library = self.library.read(cx);
        if let Some(known) = library
            .spotify
            .iter()
            .chain(library.youtube.iter())
            .chain(self.store.saved.iter())
            .find(|item| item.id == id)
        {
            return Some(Playlist {
                loading: true,
                ..known.clone()
            });
        }

        self.catalog
            .read(cx)
            .playlists
            .loading(id)
            .then(|| Playlist {
                id: id.to_owned(),
                loading: true,
                ..Default::default()
            })
    }

    pub fn songs(&self, id: &str, cx: &gpui::App) -> Vec<Song> {
        if let Some(local) = self.store.playlist(id) {
            return local.songs.clone();
        }

        self.catalog
            .read(cx)
            .playlists
            .value(id)
            .map(|playlist| playlist.songs.clone())
            .unwrap_or_default()
    }

    pub fn rows(&mut self, id: &str, cx: &gpui::App) -> Table {
        let key = format!(
            "playlist|{id}|{}|{:?}|{}|{}",
            self.ui.playlist_query,
            self.ui.playlist_order,
            self.song_count(id, cx),
            self.ui.library_revision
        );
        if self.ui.table.key.as_deref() != Some(key.as_str()) {
            let table = Table::build(
                self.songs(id, cx),
                &self.ui.playlist_query,
                self.ui.playlist_order,
                true,
            );
            self.ui.table = Table {
                key: Some(key),
                ..table
            };
        }

        self.ui.table.clone()
    }

    /// The row count the table key and the drag geometry need, without cloning
    /// the songs they would otherwise have to walk.
    pub(crate) fn song_count(&self, id: &str, cx: &gpui::App) -> usize {
        if let Some(local) = self.store.playlist(id) {
            return local.songs.len();
        }

        self.catalog
            .read(cx)
            .playlists
            .value(id)
            .map_or(0, |playlist| playlist.songs.len())
    }

    pub fn playlist(&self, id: &str, cx: &gpui::App) -> Option<Playlist> {
        let mut playlist = self.view(id, cx)?;
        playlist.songs = self.songs(id, cx);
        Some(playlist)
    }

    pub fn home_playlists(&self, cx: &gpui::App) -> Vec<Playlist> {
        let items = self.shelf_items(cx);

        match self.ui.filter {
            Filter::All => items,
            Filter::YouTube => items
                .into_iter()
                .filter(|playlist| playlist.source == Source::YouTube)
                .collect(),
            Filter::Spotify => items
                .into_iter()
                .filter(|playlist| playlist.source == Source::Spotify)
                .collect(),
            Filter::Local => items
                .into_iter()
                .filter(|playlist| playlist.source == Source::Local)
                .collect(),
        }
    }

    pub(crate) fn shelf_items(&self, cx: &gpui::App) -> Vec<Playlist> {
        let library = self.library.read(cx);
        let own: HashSet<&str> = library
            .spotify
            .iter()
            .chain(library.youtube.iter())
            .map(|playlist| playlist.id.as_str())
            .collect();
        let mut items: Vec<Playlist> = library.spotify.to_vec();
        items.extend(library.youtube.iter().cloned());

        items.extend(
            self.store
                .saved
                .iter()
                .filter(|playlist| !own.contains(playlist.id.as_str()))
                .cloned(),
        );

        items.extend(self.store.playlists.iter().map(|local| {
            let covers = local.covers();
            Playlist {
                id: local.id.clone(),
                name: local.name.clone(),
                covers,
                source: Source::Local,
                track_count: local.songs.len() as u32,
                ..Default::default()
            }
        }));

        self.order_items(items)
    }

    pub(crate) fn order_items(&self, items: Vec<Playlist>) -> Vec<Playlist> {
        let mut remaining = items;
        let mut ordered = Vec::with_capacity(remaining.len());
        for id in &self.store.order {
            if let Some(position) = remaining.iter().position(|item| &item.id == id) {
                ordered.push(remaining.remove(position));
            }
        }

        ordered.extend(remaining);
        ordered
    }

    pub(crate) fn shelf_ids(&self, cx: &gpui::App) -> Vec<String> {
        self.shelf_items(cx)
            .into_iter()
            .map(|playlist| playlist.id)
            .collect()
    }
}
