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

                    let sync = matches!(purpose, Purpose::Reload);
                    match backup::restore_staged(&mut self.store, &dir, sync) {
                        Ok(stats) => {
                            self.persist(cx);
                            self.sync_sessions(cx);
                            self.notify(
                                t("Backup reloaded · {} changes")
                                    .replace("{}", &stats.changed().to_string()),
                            );
                        }
                        Err(error) => self.fail(error),
                    }
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

                    match backup::stage(&mut self.store, &dir, meta.last_modified, trust) {
                        Ok((_, true)) => self.saf.commit(&uri),
                        Ok((_, false)) if self.store.sync_status.stale && !trust => {
                            self.saf.stage(&uri, Purpose::Pass { trust: true });
                        }
                        Ok((stats, false)) => {
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
                    }
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
}
