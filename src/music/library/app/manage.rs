use crate::shared::i18n::t;
use crate::shared::music::domain::Song;
use crate::shell::app::{Jondo, Route};

impl Jondo {
    pub fn create_playlist(&mut self, name: &str, cx: &mut gpui::Context<Self>) {
        let name = name.trim();
        if name.is_empty() {
            return;
        }

        let id = self.store.create_playlist(name);
        self.library_changed(cx);
        self.open(Route::Playlist(id), cx);
    }

    /// The local playlists, so a caller outside the crate can read what the
    /// store holds without reaching into it.
    pub fn local_playlists(&self) -> &[crate::shared::storage::LocalPlaylist] {
        &self.store.playlists
    }

    pub fn delete_playlist(&mut self, id: &str, cx: &mut gpui::Context<Self>) {
        self.store.delete_playlist(id);
        self.catalog
            .update(cx, |catalog, _| catalog.playlists.remove(id));
        self.library_changed(cx);
        if self.route == Route::Playlist(id.to_owned()) {
            self.open(Route::Home, cx);
        }
    }

    pub fn rename_playlist(&mut self, id: &str, name: &str, cx: &mut gpui::Context<Self>) {
        let name = name.trim();
        if name.is_empty() {
            return;
        }

        if let Some(playlist) = self.store.playlist_mut(id) {
            playlist.name = name.to_owned();
            playlist.modified = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|value| value.as_secs())
                .unwrap_or_default();
            self.library_changed(cx);
        }
    }

    pub fn add_to_playlist(
        &mut self,
        playlist_id: &str,
        song: &Song,
        cx: &mut gpui::Context<Self>,
    ) {
        if self.store.add_song(playlist_id, song.clone()) {
            self.touch_library();
            self.library_changed(cx);
            self.notify(t("Added to playlist"));
        } else {
            self.fail(t("Song already in playlist"));
        }
    }

    pub fn reorder_song(
        &mut self,
        playlist_id: &str,
        from: usize,
        to: usize,
        cx: &mut gpui::Context<Self>,
    ) {
        if from == to {
            return;
        }

        let Some(playlist) = self.store.playlist_mut(playlist_id) else {
            return;
        };

        if from >= playlist.songs.len() || to >= playlist.songs.len() {
            return;
        }

        let song = playlist.songs.remove(from);
        playlist.songs.insert(to, song);
        playlist.modified = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|value| value.as_secs())
            .unwrap_or_default();
        self.touch_library();
        self.library_changed(cx);
    }

    pub fn remove_song(&mut self, playlist_id: &str, song_id: &str, cx: &mut gpui::Context<Self>) {
        if let Some(playlist) = self.store.playlist_mut(playlist_id) {
            playlist.songs.retain(|song| song.id != song_id);
            self.touch_library();
            self.library_changed(cx);
        }
    }
}
