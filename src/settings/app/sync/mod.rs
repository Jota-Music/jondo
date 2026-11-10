#[cfg(not(target_os = "android"))]
use std::path::Path;
use std::time::Instant;

use crate::shared::i18n::{self, t};
#[cfg(not(target_os = "android"))]
use crate::shared::platform::desktop::dialog;

use super::backup;
#[cfg(not(target_os = "android"))]
use crate::shell::app::Event;
#[cfg(target_os = "android")]
use crate::shell::app::PendingSaf;
use crate::shell::app::{Jondo, SYNC_DEBOUNCE, SYNC_MAX_WAIT, SYNC_WATCH, SpotifyState};

mod step;

impl Jondo {
    pub(crate) fn library_changed(&mut self, cx: &gpui::App) {
        self.store.sync_pending = true;
        self.touch_library();
        self.persist(cx);
    }

    #[cfg(not(target_os = "android"))]
    pub(crate) fn backup_now(
        &mut self,
        revision: u64,
        store: crate::shared::storage::Store,
        result: Result<backup::Stats, String>,
        cx: &mut gpui::Context<Self>,
    ) {
        self.backup_running = false;
        let Some((store, changed)) = accept_backup(self.store_revision, revision, store, &result)
        else {
            // The library moved while the pass ran. A good pass starts again
            // from the newer store; a failed one waits for the next watch.
            if result.is_ok() {
                self.start_backup(cx);
            }

            return;
        };

        if let Err(error) = &result {
            log::warn!("backup: {error}");
        }

        self.store = store;
        self.save_store();
        if changed {
            self.touch_library();
        }

        cx.notify();
    }

    #[cfg(not(target_os = "android"))]
    pub(crate) fn tick_backup(&mut self, cx: &gpui::App) -> bool {
        if !self.store.sync_auto || self.store.sync_folder.trim().is_empty() || self.backup_running
        {
            return false;
        }

        let now = Instant::now();
        if self.store.sync_pending {
            let first = *self.sync_first.get_or_insert(now);
            let fire =
                (*self.sync_armed.get_or_insert(first + SYNC_DEBOUNCE)).min(first + SYNC_MAX_WAIT);
            if now >= fire {
                self.sync_first = None;
                self.sync_armed = None;
                self.sync_watch = Some(now + SYNC_WATCH);
                return self.start_backup(cx);
            }

            return false;
        }

        self.sync_first = None;
        self.sync_armed = None;
        let watch = *self.sync_watch.get_or_insert(now + SYNC_WATCH);
        if now >= watch {
            self.sync_watch = Some(now + SYNC_WATCH);
            return self.start_backup(cx);
        }

        false
    }

    /// A pass always runs in the background: the folder can live on a network
    /// mount, and its stat, read and write must never hold a frame. The settle
    /// check is the pass's own first step.
    #[cfg(not(target_os = "android"))]
    pub(crate) fn start_backup(&mut self, _cx: &gpui::App) -> bool {
        if self.store.sync_folder.trim().is_empty() {
            return false;
        }

        self.backup_running = true;
        let revision = self.store_revision;
        let mut snapshot = self.store.clone();
        self.jobs.blocking(move || {
            let result = backup::auto(&mut snapshot);
            Event::Backup(revision, Box::new(snapshot), result)
        });
        false
    }

    /// Reads the document once so the sessions sealed with a fresh pass reach
    /// this device, without waiting for the next save.
    pub(crate) fn reimport(&mut self, cx: &gpui::App) {
        if self.store.sync_folder.trim().is_empty() {
            return;
        }

        #[cfg(target_os = "android")]
        {
            let _ = cx;
            self.start_pass(true);
        }

        #[cfg(not(target_os = "android"))]
        {
            if !self.backup_running {
                self.start_backup(cx);
            }
        }
    }

    /// A save bumps the revision the async backup compares against, so a pass
    /// that started before this write cannot replace the newer library.
    pub(crate) fn save_store(&mut self) {
        self.store_revision = self.store_revision.wrapping_add(1);
        self.store.save();
    }

    #[cfg(target_os = "android")]
    pub(crate) fn tick_android(&mut self, cx: &mut gpui::Context<Self>) -> bool {
        crate::shell::app::platform::media::sync(self, cx);
        if let Some(step) = self.saf.poll() {
            self.step(step, cx);
            return true;
        }

        if self.saf.busy() {
            return false;
        }

        if !self.store.sync_auto || self.store.sync_folder.trim().is_empty() {
            return false;
        }

        let now = Instant::now();
        if self.store.sync_pending {
            let first = *self.sync_first.get_or_insert(now);
            let fire =
                (*self.sync_armed.get_or_insert(first + SYNC_DEBOUNCE)).min(first + SYNC_MAX_WAIT);
            if now >= fire {
                self.sync_first = None;
                self.sync_armed = None;
                self.sync_watch = Some(now + SYNC_WATCH);
                self.start_pass(false);
            }

            return false;
        }

        self.sync_first = None;
        self.sync_armed = None;
        let watch = *self.sync_watch.get_or_insert(now + SYNC_WATCH);
        if now >= watch {
            self.sync_watch = Some(now + SYNC_WATCH);
            self.start_pass(false);
        }

        false
    }

    #[cfg(target_os = "android")]
    pub(super) fn start_pass(&mut self, trust: bool) {
        let uri = self.store.sync_folder.trim().to_owned();
        self.saf
            .info(&uri, crate::shared::platform::saf::Purpose::Pass { trust });
    }

    #[cfg(target_os = "android")]
    pub fn pick_backup(&mut self, pick: crate::shared::platform::saf::Pick) {
        self.pending_saf = None;
        let last = self.store.sync_folder.trim().to_owned();
        self.saf.pick(pick, &last);
    }

    pub fn backup(&mut self, mode: backup::Mode, cx: &gpui::App) {
        #[cfg(target_os = "android")]
        let _ = cx;
        if self.store.sync_folder.trim().is_empty() {
            self.fail(t("Set a backup location first"));
            return;
        }

        #[cfg(target_os = "android")]
        {
            match mode {
                backup::Mode::Newer => self.start_pass(true),
                backup::Mode::Restore => {
                    let uri = self.store.sync_folder.trim().to_owned();
                    self.saf
                        .stage(&uri, crate::shared::platform::saf::Purpose::Reload);
                }
            }

            return;
        }

        #[cfg(not(target_os = "android"))]
        let result = match mode {
            backup::Mode::Restore => backup::restore(&mut self.store),
            backup::Mode::Newer => backup::save(&mut self.store),
        };

        #[cfg(not(target_os = "android"))]
        match result {
            Ok(stats) => {
                self.persist(cx);
                let label = match mode {
                    backup::Mode::Restore => t("Backup reloaded"),
                    backup::Mode::Newer => t("Backup saved"),
                };

                if stats.changed() > 0 {
                    self.notify(format!("{label} · {} changes", stats.changed()));
                } else {
                    self.notify(label);
                }
            }
            Err(error) => self.fail(error),
        }
    }

    pub fn overwrite_backup(&mut self, cx: &gpui::App) {
        #[cfg(target_os = "android")]
        let _ = cx;
        if self.store.sync_folder.trim().is_empty() {
            self.fail(t("Set a backup location first"));
            return;
        }

        #[cfg(target_os = "android")]
        {
            self.start_pass(true);
            return;
        }

        #[cfg(not(target_os = "android"))]
        match backup::overwrite(&mut self.store) {
            Ok(stats) => {
                self.persist(cx);
                if stats.changed() > 0 {
                    self.notify(
                        t("Backup saved").to_owned()
                            + " · "
                            + &stats.changed().to_string()
                            + " changes",
                    );
                } else {
                    self.notify(t("Backup saved"));
                }
            }
            Err(error) => self.fail(error),
        }
    }

    pub(crate) fn touch_library(&mut self) {
        self.ui.library_revision = self.ui.library_revision.wrapping_add(1);
    }

    pub fn sync_sessions(&mut self, cx: &mut gpui::Context<Self>) -> bool {
        let mut changed = false;
        if self.settings.read(cx).youtube_applied != self.store.youtube_cookies {
            self.settings.update(cx, |settings, _| {
                settings.youtube_applied = self.store.youtube_cookies.clone()
            });
            self.youtube.set_session(&self.store.youtube_cookies);
            if self.youtube.signed_in() {
                self.load_youtube_library(cx);
            } else {
                self.library
                    .update(cx, |library, _| library.youtube.clear());
            }

            changed = true;
        }

        let token = self
            .store
            .spotify
            .as_ref()
            .map(|auth| auth.data.clone())
            .unwrap_or_default();
        if token != self.settings.read(cx).applied {
            self.settings
                .update(cx, |settings, _| settings.applied = token);
            if self.store.spotify.is_some()
                && self.settings.read(cx).spotify != SpotifyState::Connecting
            {
                self.resume_spotify(cx);
                changed = true;
            }
        }

        changed
    }

    pub fn set_locale(&mut self, locale: i18n::Locale, cx: &gpui::App) {
        i18n::set(locale);
        self.store.locale = locale.code().to_owned();
        self.persist(cx);
    }

    #[cfg(target_os = "android")]
    pub fn export_backup(&mut self) {
        // Jota hands the phone the file through the share sheet: there is no
        // system dialog that writes a JSON file wherever the user wants it.
        let Some(dir) = crate::shared::platform::android::data_dir().map(|dir| dir.join("backup"))
        else {
            self.fail(t("No folder to save the backup"));
            return;
        };

        if let Err(error) = std::fs::create_dir_all(&dir) {
            self.fail(error.to_string());
            return;
        }

        let path = dir.join(backup::FILE);
        match backup::export(&mut self.store, &path) {
            Ok(_) => {
                match crate::shared::platform::android::share::send(
                    &path,
                    "application/json",
                    t("Export backup"),
                ) {
                    Ok(()) => self.notify(t("Backup exported")),
                    Err(error) => self.fail(error),
                }
            }
            Err(error) => self.fail(error),
        }
    }

    #[cfg(not(target_os = "android"))]
    pub fn export_backup(&mut self) {
        self.jobs.blocking(|| {
            Event::ExportPath(dialog(|file| {
                file.set_title(t("Export backup"))
                    .set_file_name(backup::FILE)
                    .add_filter("JSON", &["json"])
                    .save_file()
            }))
        });
    }

    #[cfg(target_os = "android")]
    pub fn import_backup(&mut self) {
        self.pending_saf = Some(PendingSaf::Import);
        self.saf
            .pick(crate::shared::platform::saf::Pick::Document, "");
    }

    #[cfg(not(target_os = "android"))]
    pub fn import_backup(&mut self) {
        self.jobs.blocking(|| {
            Event::ImportPath(dialog(|file| {
                file.set_title(t("Import backup"))
                    .add_filter("JSON", &["json"])
                    .pick_file()
            }))
        });
    }

    #[cfg(not(target_os = "android"))]
    pub fn choose_folder(&mut self) {
        self.jobs.blocking(|| {
            Event::Folder(dialog(|file| {
                file.set_title(t("Choose the sync folder")).pick_folder()
            }))
        });
    }

    #[cfg(target_os = "android")]
    pub fn choose_folder(&mut self) {
        self.pick_backup(crate::shared::platform::saf::Pick::Folder);
    }

    #[cfg(not(target_os = "android"))]
    pub(crate) fn adopt_folder(&mut self, path: &Path, cx: &gpui::App) {
        self.store.sync_folder = path.display().to_string();
        self.persist(cx);
        self.notify(t("Backup location saved"));
        self.backup(backup::Mode::Newer, cx);
    }

    #[cfg(not(target_os = "android"))]
    pub(crate) fn write_backup(&mut self, path: &Path, cx: &gpui::App) {
        match backup::export(&mut self.store, path) {
            Ok(_) => {
                self.persist(cx);
                self.notify(t("Backup exported to {}").replace("{}", &path.display().to_string()));
            }
            Err(error) => self.fail(error),
        }
    }

    #[cfg(not(target_os = "android"))]
    pub(crate) fn read_backup(&mut self, path: &Path, cx: &mut gpui::Context<Self>) {
        match backup::import(&mut self.store, path) {
            Ok(stats) => {
                self.persist(cx);
                self.sync_sessions(cx);
                self.notify(
                    t("Backup restored · {} changes").replace("{}", &stats.changed().to_string()),
                );
            }
            Err(error) => self.fail(error),
        }
    }

    pub fn save_logs(&mut self) {
        match crate::shared::logs::save() {
            Ok(path) => {
                #[cfg(target_os = "android")]
                {
                    let _ = path;
                    self.notify(t("Logs shared"));
                }

                #[cfg(not(target_os = "android"))]
                self.notify(t("Logs saved to {}").replace("{}", &path.display().to_string()));
            }
            Err(error) => self.fail(error),
        }
    }
}

/// A backup may replace the store only when nothing was saved after the pass
/// started. The changed flag drives the library caches, not the swap.
#[cfg(not(target_os = "android"))]
pub(crate) fn accept_backup(
    current: u64,
    started: u64,
    store: crate::shared::storage::Store,
    result: &Result<backup::Stats, String>,
) -> Option<(crate::shared::storage::Store, bool)> {
    (current == started).then(|| {
        let changed = result.as_ref().is_ok_and(|stats| stats.changed() > 0);
        (store, changed)
    })
}

#[cfg(test)]
#[path = "sync.test.rs"]
mod tests;
