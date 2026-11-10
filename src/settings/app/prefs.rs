use crate::shell::app::Jondo;

/// Preferences the panels write. The store is the app's, so the views ask for
/// the change instead of touching it and remembering to persist it themselves.
impl Jondo {
    pub fn set_sync_auto(&mut self, on: bool, cx: &gpui::App) {
        self.store.sync_auto = on;
        self.persist(cx);
    }

    pub fn set_sync_import(&mut self, on: bool, cx: &gpui::App) {
        self.store.sync_import = on;
        self.persist(cx);
    }

    pub fn set_sync_interval(&mut self, minutes: u64, cx: &gpui::App) {
        self.store.sync_interval = minutes;
        self.persist(cx);
    }

    pub fn set_discord(&mut self, on: bool, cx: &gpui::App) {
        self.store.discord = on;
        self.persist(cx);
    }

    pub fn set_trim_memory(&mut self, on: bool, cx: &gpui::App) {
        self.store.trim_memory = on;
        self.persist(cx);
    }

    pub fn set_pause_on_disconnect(&mut self, on: bool, cx: &gpui::App) {
        self.store.pause_on_disconnect = on;
        self.persist(cx);
    }

    pub fn set_pause_on_focus(&mut self, on: bool, cx: &gpui::App) {
        self.store.pause_on_focus = on;
        self.persist(cx);
    }

    pub fn set_session_pass(&mut self, pass: String, cx: &gpui::App) {
        if pass == self.store.session_pass {
            return;
        }

        self.store.session_pass = pass;
        self.library_changed(cx);
        if !self.store.session_pass.trim().is_empty() {
            self.reimport(cx);
        }
    }
}
