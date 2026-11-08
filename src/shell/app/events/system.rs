use crate::shared::i18n::t;

use super::*;

impl Jondo {
    pub(super) fn system_event(&mut self, event: Event, cx: &mut gpui::Context<Self>) {
        match event {
            Event::ClipboardSong(result) => {
                self.clipboard.busy = false;
                match result {
                    Ok(song) => {
                        self.clipboard.pending = None;
                        self.play_now(song, cx);
                    }
                    Err(error) => self.fail(error),
                }
            }
            Event::Room(event) => self.room_event(*event, cx),
            Event::Update(result) => {
                self.settings
                    .update(cx, |settings, _| settings.update = result.ok());
            }
            Event::Installed(result) => {
                self.ui.updating = false;
                match result {
                    Ok(path) => {
                        #[cfg(target_os = "android")]
                        match crate::shared::platform::android::share::install(&path) {
                            Ok(()) => self.notify(t("Confirm the install on your phone.")),
                            Err(error) => self.fail(error),
                        }

                        #[cfg(not(target_os = "android"))]
                        {
                            let _ = path;
                            self.shutdown(cx);
                            cx.quit();
                        }
                    }
                    Err(error) => self.fail(error),
                }
            }
            Event::Browsers(browsers) => {
                self.settings
                    .update(cx, |settings, _| settings.browsers = browsers);
                self.settings
                    .update(cx, |settings, _| settings.browsers_checked = true);
                self.settings
                    .update(cx, |settings, _| settings.browsers_loading = false);
                if !self
                    .settings
                    .read(cx)
                    .browsers
                    .iter()
                    .any(|name| *name == self.ui.browser_pick)
                {
                    self.ui.browser_pick = self
                        .settings
                        .read(cx)
                        .browsers
                        .first()
                        .map(|name| (*name).to_owned())
                        .unwrap_or_default();
                }
            }
            Event::BrowserCookies(browser, result) => {
                self.ui.cookies_importing = false;
                match result {
                    Ok(cookies) => {
                        if self.adopt_cookies(&cookies, cx) {
                            self.store.youtube_browser = browser.to_owned();
                            self.persist(cx);
                        }
                    }
                    Err(error) => {
                        self.ui.cookies_error =
                            Some(t("Could not use the {} session").replace("{}", browser));
                        log::warn!("youtube: {browser}: {error}");
                    }
                }
            }
            Event::Rescue(rescued) => {
                self.settings
                    .update(cx, |settings, _| settings.youtube_rescue = false);
                match rescued {
                    Some((browser, cookies)) => {
                        if self.adopt_cookies(&cookies, cx) {
                            self.store.youtube_browser = browser.to_owned();
                            self.settings
                                .update(cx, |settings, _| settings.youtube_expired = false);
                            self.persist(cx);
                        }
                    }
                    None if self.settings.update(cx, |settings, _| {
                        std::mem::take(&mut settings.youtube_expired)
                    }) =>
                    {
                        self.notify(t("The YouTube session expired. Import it again."));
                    }
                    None => {}
                }
            }
            Event::BrowserRefresh(retry, result) => match result {
                Ok(cookies) => {
                    let refreshed = self.refresh_cookies(&cookies, cx);
                    if retry && !refreshed {
                        self.expire_youtube(cx);
                    }
                }
                Err(error) => {
                    log::info!("youtube: browser read: {error}");
                    if retry {
                        self.notify(t("The YouTube session expired. Import it again."));
                    }
                }
            },
            Event::CookiesFile(result) => {
                self.ui.cookies_importing = false;
                match result {
                    Ok(text) if text.trim().is_empty() => {}
                    Ok(text) => {
                        self.adopt_cookies(&text, cx);
                    }
                    Err(error) => self.ui.cookies_error = Some(error),
                }
            }
            _ => {}
        }
    }
}
