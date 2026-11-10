use crate::music::player::repeat_name;

use super::Jondo;

impl Jondo {
    pub fn shutdown(&mut self, cx: &gpui::Context<Self>) {
        self.store.sync_pending = true;
        self.persist(cx);
        self.store.save_now();
        self.jobs.stop();
    }

    /// The app-wide snapshot written before a save. It composes every feature
    /// the store owns, so it lives in the shell, not in settings.
    pub fn persist(&mut self, cx: &gpui::App) {
        let player = self.player.read(cx);
        self.store.volume = player.volume;
        self.store.muted = player.muted;
        self.store.queue = player.queue.songs.clone();
        self.store.queue_index = player.queue.at;
        self.store.position = player.progress;
        self.store.shuffle = player.queue.shuffle;
        self.store.repeat = repeat_name(player.repeat).to_owned();
        self.store.compact = self.ui.compact;
        if !self.rooms.read(cx).relay_locked {
            self.store.relay_url = self.rooms.read(cx).relay.trim().to_owned();
            self.store.relay_token = self.rooms.read(cx).token.trim().to_owned();
        }

        self.store.room_code = self.rooms.read(cx).code.clone();
        self.store.room_password = self.rooms.read(cx).password.clone();
        self.save_store();
    }
}
