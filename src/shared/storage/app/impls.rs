use std::fs;
use std::io::Write;

use crate::shared::music::domain::Room;
use crate::shared::music::domain::Song;

use crate::shared::kernel::app::clock::{now, now_ms};
use crate::shared::storage::domain::*;

use super::file::*;

impl Store {
    pub fn load() -> Self {
        let Some(path) = path() else {
            return Self::default();
        };

        match fs::read_to_string(&path) {
            Ok(text) => {
                let mut store: Self = serde_json::from_str(&text).unwrap_or_else(|error| {
                    log::warn!("cannot read {}: {error}", path.display());
                    Self::default()
                });
                store.normalize();
                store
            }
            Err(_) => Self::default(),
        }
    }

    /// Brings the local follow list to the key format Jota stores, so a follow
    /// written by either app reads the same here.
    pub(super) fn normalize(&mut self) {
        if self.follows.is_empty() {
            return;
        }

        let mut times = std::mem::take(&mut self.follows_at);
        let mut follows = Vec::with_capacity(self.follows.len());
        for mut follow in std::mem::take(&mut self.follows) {
            let old = follow.id.clone();
            let target = follow.target().to_owned();
            if target.is_empty() {
                continue;
            }

            follow.id = crate::shared::music::domain::Follow::key(follow.kind, &target);
            if follow.name.is_empty() || follow.name == old {
                follow.name = target;
            }

            if follow.source.is_none() {
                follow.source = Some(match follow.kind {
                    crate::shared::music::domain::FollowKind::Channel => {
                        crate::shared::music::domain::Source::YouTube
                    }
                    _ => crate::shared::music::domain::Source::Spotify,
                });
            }

            if let Some(at) = times.remove(&old) {
                self.follows_at.insert(follow.id.clone(), at);
            }

            follows.push(follow);
        }

        self.follows = follows;
        self.follows_at.extend(times);
        let graves = std::mem::take(&mut self.follow_graves);
        for (id, at) in graves {
            let key = crate::shared::music::domain::Follow::canonical(&id).id;
            let entry = self.follow_graves.entry(key).or_default();
            if at > *entry {
                *entry = at;
            }
        }
    }

    pub fn link(&mut self, id: &str, video: &str) -> bool {
        self.set_link(id, video, false)
    }

    pub fn pick(&mut self, id: &str, video: &str) -> bool {
        self.set_link(id, video, true)
    }

    pub(super) fn set_link(&mut self, id: &str, video: &str, manual: bool) -> bool {
        if id.is_empty() {
            return false;
        }

        if video.is_empty() {
            self.youtube_picks.remove(id);
            return self.youtube_links.remove(id).is_some();
        }

        if id.strip_prefix("youtube:") == Some(video) {
            return false;
        }

        if !manual && self.youtube_picks.contains(id) {
            return false;
        }

        let mut changed = false;
        if self.youtube_links.get(id).map(String::as_str) != Some(video) {
            self.youtube_links.insert(id.to_owned(), video.to_owned());
            changed = true;
        }

        if manual && self.youtube_picks.insert(id.to_owned()) {
            changed = true;
        }

        changed
    }

    /// Hands the state to the writer thread, which keeps only the newest copy
    /// waiting. A frame never waits for a disk flush.
    pub fn save(&self) {
        let slot = writer();
        if let Ok(mut waiting) = slot.store.lock() {
            *waiting = Some(self.clone());
            slot.ready.notify_one();
        } else {
            self.write();
        }
    }

    /// Writes now, for the paths that end the process.
    pub fn save_now(&self) {
        self.write();
    }

    pub(super) fn write(&self) {
        let Some(path) = path() else {
            return;
        };

        if let Some(parent) = path.parent()
            && let Err(error) = fs::create_dir_all(parent)
        {
            log::warn!("cannot create {}: {error}", parent.display());
            return;
        }

        let Ok(text) = serde_json::to_string(self) else {
            return;
        };

        let temp = path.with_extension("tmp");
        let write = || -> std::io::Result<()> {
            let mut file = fs::File::create(&temp)?;
            file.write_all(text.as_bytes())?;
            file.sync_all()?;
            fs::rename(&temp, &path)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
            }

            Ok(())
        };

        if let Err(error) = write() {
            log::warn!("cannot save {}: {error}", path.display());
        }
    }

    pub fn playlist(&self, id: &str) -> Option<&LocalPlaylist> {
        self.playlists.iter().find(|playlist| playlist.id == id)
    }

    pub fn playlist_mut(&mut self, id: &str) -> Option<&mut LocalPlaylist> {
        self.playlists.iter_mut().find(|playlist| playlist.id == id)
    }

    pub fn create_playlist(&mut self, name: &str) -> String {
        let id = crate::shared::music::domain::local_id();
        self.playlists.insert(
            0,
            LocalPlaylist {
                id: id.clone(),
                name: name.to_owned(),
                songs: Vec::new(),
                modified: now(),
            },
        );
        id
    }

    pub fn delete_playlist(&mut self, id: &str) {
        self.playlists.retain(|playlist| playlist.id != id);
        self.playlist_graves.insert(id.to_owned(), now_ms());
    }

    pub fn add_song(&mut self, playlist_id: &str, song: Song) -> bool {
        let Some(playlist) = self.playlist_mut(playlist_id) else {
            return false;
        };

        if playlist.songs.iter().any(|existing| existing.id == song.id) {
            return false;
        }

        playlist.songs.push(song);
        playlist.modified = now();
        true
    }

    pub fn save_room(&mut self, mut room: Room) {
        room.code = room.code.trim().to_owned();
        room.relay_url = room.relay_url.trim().to_owned();
        room.name = room.name.trim().to_owned();
        if room.code.is_empty() || room.relay_url.is_empty() {
            return;
        }

        if room.name.is_empty() {
            room.name = room.code.clone();
        }

        if room.saved_at == 0 {
            room.saved_at = now();
        }

        room.id = crate::shared::music::domain::room_id(&room.relay_url, &room.code);
        if let Some(existing) = self.rooms.iter_mut().find(|item| item.id == room.id) {
            *existing = room;
        } else {
            self.rooms.push(room);
        }
    }

    pub fn remove_room(&mut self, id: &str) {
        self.rooms.retain(|room| room.id != id);
        self.room_order.retain(|item| item != id);
    }

    /// The saved rooms in the order the shelf shows them. Rooms saved after the
    /// last reorder have no entry yet and keep their insertion order at the end.
    pub fn rooms_ordered(&self) -> Vec<Room> {
        let mut remaining = self.rooms.clone();
        let mut ordered = Vec::with_capacity(remaining.len());
        for id in &self.room_order {
            if let Some(position) = remaining.iter().position(|room| &room.id == id) {
                ordered.push(remaining.remove(position));
            }
        }

        ordered.extend(remaining);
        ordered
    }
}
