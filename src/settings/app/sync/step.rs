#[cfg(target_os = "android")]
use crate::shell::app::Event;
use crate::shell::app::Jondo;

impl Jondo {
    #[cfg(target_os = "android")]
    pub(super) fn step(
        &mut self,
        step: crate::shared::platform::saf::Step,
        cx: &mut gpui::Context<Self>,
    ) {
        use crate::settings::app::backup;
        use crate::shared::i18n::t;
        use crate::shared::platform::saf::{Purpose, Step};
        use crate::shell::app::PendingSaf;

        let uri = self.store.sync_folder.trim().to_owned();
        match step {
            Step::Infoed(purpose, result) => match result {
                Ok(meta) => {
                    let trust = matches!(purpose, Purpose::Reload | Purpose::Pass { trust: true });
                    if !trust
                        && backup::settled_document(
                            &self.store,
                            meta.as_ref().map_or(0, |meta| meta.last_modified),
                            meta.as_ref().map_or(-1, |meta| meta.size),
                        )
                    {
                        backup::touch(&mut self.store);
                        self.persist(cx);
                        return;
                    }

                    self.saf.stage(&uri, purpose);
                }
                Err(error) => {
                    backup::failed(&mut self.store, &error);
                    self.fail(t("Cannot read the backup location"));
                }
            },
            Step::Staged(purpose @ (Purpose::Reload | Purpose::Manual), result) => match result {
                Ok(meta) => {
                    self.store.sync_status.name = meta.name;
                    let Some(dir) = backup::stage_dir() else {
                        self.fail(t("No place to stage the backup"));
                        return;
                    };

                    self.merge(dir, purpose, uri, 0, cx);
                }
                Err(error) => {
                    backup::failed(&mut self.store, &error);
                    self.fail(t("Cannot read the backup file"));
                }
            },
            Step::Staged(Purpose::Pass { trust }, result) => match result {
                Ok(meta) => {
                    self.store.sync_status.name = meta.name;
                    let Some(dir) = backup::stage_dir() else {
                        self.fail(t("No place to stage the backup"));
                        return;
                    };

                    self.merge(dir, Purpose::Pass { trust }, uri, meta.last_modified, cx);
                }
                Err(error) => {
                    backup::failed(&mut self.store, &error);
                    self.fail(t("Cannot read the backup file"));
                }
            },
            Step::Committed(result) => match result {
                Ok(()) => {
                    if let Some(dir) = backup::stage_dir() {
                        backup::committed(&mut self.store, &dir);
                    }

                    self.persist(cx);
                    self.notify(t("Backup saved"));
                }
                Err(error) => {
                    backup::failed(&mut self.store, &error);
                    self.fail(t("Cannot write the backup file"));
                }
            },
            Step::Picked(result) => match (self.pending_saf, result) {
                (_, Ok(choice)) if choice.uri.is_empty() => {
                    self.pending_saf = None;
                }
                (Some(PendingSaf::Import), Ok(choice)) => {
                    self.pending_saf = None;
                    self.saf.stage(&choice.uri, Purpose::Manual);
                }
                (_, Ok(choice)) => {
                    let duplicate =
                        choice.created && !choice.name.is_empty() && choice.name != backup::FILE;
                    self.store.sync_folder = choice.uri;
                    self.store.sync_status.name = choice.name;
                    self.persist(cx);
                    if duplicate {
                        self.notify(t("The provider created a new file instead of using the one that already existed. There are two backups now: pick the right one."));
                    } else {
                        self.notify(t("Backup location saved"));
                    }

                    self.start_pass(true);
                }
                (_, Err(error)) => {
                    self.pending_saf = None;
                    self.fail(error);
                }
            },
            Step::Lost => self.fail(t("The backup call failed")),
        }
    }

    /// The merge reads the staged document and seals or opens the sessions it
    /// carries, so it belongs on the pool: the frame only applies the answer.
    #[cfg(target_os = "android")]
    fn merge(
        &mut self,
        dir: std::path::PathBuf,
        purpose: crate::shared::platform::saf::Purpose,
        uri: String,
        stamp: i64,
        cx: &mut gpui::Context<Self>,
    ) {
        let revision = self.store_revision;
        let mut store = self.store.clone();
        self.backup_running = true;
        self.jobs.blocking(move || {
            let result = match purpose {
                crate::shared::platform::saf::Purpose::Pass { trust } => {
                    crate::settings::app::backup::stage(&mut store, &dir, stamp, trust)
                        .map(|(stats, _)| stats)
                }
                purpose => {
                    let sync = matches!(purpose, crate::shared::platform::saf::Purpose::Reload);
                    crate::settings::app::backup::restore_staged(&mut store, &dir, sync)
                }
            };

            Event::Staged {
                revision,
                uri,
                purpose,
                store: Box::new(store),
                result,
            }
        });
        cx.notify();
    }

    #[cfg(target_os = "android")]
    pub(crate) fn staged(
        &mut self,
        revision: u64,
        uri: String,
        purpose: crate::shared::platform::saf::Purpose,
        store: crate::shared::storage::Store,
        result: Result<crate::settings::app::backup::Stats, String>,
        cx: &mut gpui::Context<Self>,
    ) {
        use crate::settings::app::backup;
        use crate::shared::i18n::t;
        use crate::shared::platform::saf::Purpose;

        self.backup_running = false;
        if revision != self.store_revision {
            // The library moved while the merge ran. A good pass starts again
            // from the newer store; a failed one waits for the next watch.
            if let Err(error) = result {
                backup::failed(&mut self.store, &error);
            }

            match purpose {
                Purpose::Pass { trust } => self.start_pass(trust),
                Purpose::Reload => self.saf.stage(&uri, Purpose::Reload),
                Purpose::Manual => {
                    self.notify(t("The library changed during the import. Import it again."))
                }
            }

            return;
        }

        self.store = store;
        match purpose {
            Purpose::Pass { trust } => match result {
                Ok(stats) if stats.modified => self.saf.commit(&uri),
                Ok(_) if self.store.sync_status.stale && !trust => {
                    self.saf.stage(&uri, Purpose::Pass { trust: true });
                }
                Ok(stats) => {
                    self.persist(cx);
                    if stats.changed() > 0 {
                        self.notify(
                            t("Backup merged · {} changes")
                                .replace("{}", &stats.changed().to_string()),
                        );
                    }
                }
                Err(error) => {
                    self.fail(error);
                    self.persist(cx);
                }
            },
            _ => match result {
                Ok(stats) => {
                    self.persist(cx);
                    self.sync_sessions(cx);
                    self.notify(
                        t("Backup reloaded · {} changes")
                            .replace("{}", &stats.changed().to_string()),
                    );
                }
                Err(error) => self.fail(error),
            },
        }
    }
}
