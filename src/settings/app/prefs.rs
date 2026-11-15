use crate::shared::i18n::t;
use crate::shared::storage::domain::{GLOW_RANGE, MOTE_RANGE};
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

    pub fn set_backdrop(&mut self, on: bool, cx: &gpui::App) {
        self.store.backdrop = on;
        self.persist(cx);
    }

    /// The glow's two knobs, as the percentage their field shows. The range is
    /// the one the field reads well in, and it is held here so the field, a
    /// reset and any later caller land on the same number.
    pub fn set_glow(&mut self, percent: u8, cx: &gpui::App) {
        self.store.glow = percent.clamp(GLOW_RANGE.0, GLOW_RANGE.1);
        self.persist(cx);
    }

    pub fn set_mote(&mut self, percent: u8, cx: &gpui::App) {
        self.store.mote = percent.clamp(MOTE_RANGE.0, MOTE_RANGE.1);
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

    pub fn set_session_pass(&mut self, pass: String, cx: &mut gpui::Context<Self>) {
        if pass == self.store.session_pass {
            return;
        }

        let sealing = !pass.trim().is_empty();
        self.store.session_pass = pass;
        self.library_changed(cx);
        self.notify(if sealing {
            t("Session pass saved")
        } else {
            t("Session encryption off")
        });
        cx.notify();
        if sealing {
            self.reimport(cx);
        }
    }
}
