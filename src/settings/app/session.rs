use std::sync::Arc;
use std::time::Instant;

use crate::shared::i18n::t;
use crate::shared::music::domain::Source;
#[cfg(not(target_os = "android"))]
use crate::shared::platform::desktop::{dialog, downloads};
use crate::shared::platform::pin;
use crate::shared::spotify;
use crate::shell::app::{
    Event, Jondo, Route, SPOTIFY_RETRY, SPOTIFY_RETRY_MAX, SearchType, SpotifyState, cache,
};

impl Jondo {
    pub fn sign_in_youtube(&mut self, cx: &mut gpui::Context<Self>) {
        let draft = self.ui.cookies_draft.clone();
        self.adopt_cookies(&draft, cx);
    }

    pub(super) fn can_adopt(&self, cookies: &str) -> bool {
        !self.store.youtube_full || crate::shared::youtube::full(cookies)
    }

    pub(super) fn store_session(&mut self, cookies: &str, cx: &mut gpui::Context<Self>) {
        self.store.youtube_full = crate::shared::youtube::full(cookies);
        self.store.youtube_at = crate::shared::kernel::app::clock::now_ms();
        self.store.youtube_cookies = cookies.to_owned();
        self.settings.update(cx, |settings, _| {
            settings.youtube_applied = cookies.to_owned()
        });
        self.youtube.set_session(cookies);
        self.store.sync_pending = true;
        self.persist(cx);
    }

    pub(crate) fn adopt_cookies(&mut self, draft: &str, cx: &mut gpui::Context<Self>) -> bool {
        self.ui.cookies_error = None;
        let draft = if crate::shared::crypto::session::sealed(draft) {
            let Some(plain) =
                crate::shared::crypto::session::decrypt(&self.store.session_pass, draft).ok()
            else {
                self.ui.cookies_error =
                    Some(t("The session pass does not match that file").to_owned());
                return false;
            };

            plain
        } else {
            draft.to_owned()
        };

        let Some(cookies) = crate::shared::youtube::parse_cookies(&draft) else {
            self.ui.cookies_error = Some(t("Those cookies have no SAPISID").to_owned());
            return false;
        };

        if !self.can_adopt(&cookies) {
            self.ui.cookies_error = Some(
                t("That session only signs requests; keeping the one that opens your lists")
                    .to_owned(),
            );
            return false;
        }

        self.store_session(&cookies, cx);
        self.ui.cookies_open = false;
        self.ui.cookies_draft.clear();
        self.load_youtube_library(cx);
        self.notify(t("YouTube session imported"));
        true
    }

    pub(crate) fn refresh_browser(&mut self, retry: bool, _cx: &mut gpui::Context<Self>) {
        if self.store.youtube_browser.is_empty() {
            return;
        }

        let browser = self.store.youtube_browser.clone();
        self.jobs.blocking(move || {
            Event::BrowserRefresh(retry, crate::shared::browser::cookies(&browser))
        });
    }

    pub(crate) fn rescue(&mut self, cx: &mut gpui::Context<Self>) {
        let stored = self.store.youtube_cookies.clone();
        self.settings
            .update(cx, |settings, _| settings.youtube_rescue = true);
        self.jobs.blocking(move || {
            Event::Rescue(
                crate::shared::browser::available()
                    .into_iter()
                    .find_map(|name| {
                        let cookies = crate::shared::browser::cookies(name).ok()?;
                        crate::shared::youtube::same(&stored, &cookies).then_some((name, cookies))
                    }),
            )
        });
    }

    pub(crate) fn refresh_cookies(&mut self, raw: &str, cx: &mut gpui::Context<Self>) -> bool {
        let Some(cookies) = crate::shared::youtube::parse_cookies(raw) else {
            return false;
        };

        if !self.can_adopt(&cookies) || cookies == self.store.youtube_cookies {
            return false;
        }

        self.ui.cookies_error = None;
        self.store_session(&cookies, cx);
        self.load_youtube_library(cx);
        true
    }

    pub fn ensure_browsers(&mut self, cx: &mut gpui::Context<Self>) {
        if self.settings.read(cx).browsers_checked || self.settings.read(cx).browsers_loading {
            return;
        }

        self.settings
            .update(cx, |settings, _| settings.browsers_loading = true);
        self.jobs
            .blocking(|| Event::Browsers(crate::shared::browser::available()));
    }

    pub fn sign_in_browser(&mut self, browser: &'static str, _cx: &mut gpui::Context<Self>) {
        if browser.is_empty() || self.ui.cookies_importing {
            return;
        }

        self.ui.cookies_importing = true;
        self.ui.cookies_error = None;
        self.jobs.blocking(move || {
            Event::BrowserCookies(browser, crate::shared::browser::cookies(browser))
        });
    }

    #[cfg(not(target_os = "android"))]
    pub fn import_cookies_file(&mut self, _cx: &mut gpui::Context<Self>) {
        if self.ui.cookies_importing {
            return;
        }

        self.ui.cookies_importing = true;
        self.ui.cookies_error = None;
        self.jobs.blocking(|| {
            let picked = dialog(|file| {
                file.set_title(t("Import cookies"))
                    .add_filter("Cookies", &["txt", "enc"])
                    .pick_file()
            });
            let Some(path) = picked else {
                return Event::CookiesFile(Ok(String::new()));
            };

            Event::CookiesFile(
                std::fs::read_to_string(&path).map_err(|error| {
                    t("Cannot read that file: {}").replace("{}", &error.to_string())
                }),
            )
        });
    }

    pub(crate) fn expire_youtube(&mut self, cx: &mut gpui::Context<Self>) {
        self.youtube.set_session("");
        self.library
            .update(cx, |library, _| library.youtube.clear());
        self.settings
            .update(cx, |settings, _| settings.youtube_expired = false);
        self.notify(t("The YouTube session expired. Import it again."));
    }

    pub fn sign_out_youtube(&mut self, cx: &mut gpui::Context<Self>) {
        self.store.youtube_cookies.clear();
        self.store.youtube_full = false;
        self.store.youtube_browser.clear();
        self.store.youtube_at = crate::shared::kernel::app::clock::now_ms();
        self.youtube.set_session("");
        self.settings
            .update(cx, |settings, _| settings.youtube_applied.clear());
        self.library
            .update(cx, |library, _| library.youtube.clear());
        self.ui.cookies_draft.clear();
        self.ui.cookies_open = false;
        self.ui.cookies_error = None;
        self.store.sync_pending = true;
        self.persist(cx);
    }

    pub fn export_cookies(&mut self, _cx: &mut gpui::Context<Self>) {
        if self.store.youtube_cookies.is_empty() {
            self.fail(t("There is no YouTube session to export"));
            return;
        }

        let sealed = !self.store.session_pass.trim().is_empty();
        let (name, body) = if sealed {
            match crate::shared::crypto::session::encrypt(
                self.store.session_pass.trim(),
                &self.store.youtube_cookies,
            ) {
                Ok(sealed) => ("youtube-cookies.enc", sealed),
                Err(error) => {
                    self.fail(error);
                    return;
                }
            }
        } else {
            ("youtube-cookies.txt", self.store.youtube_cookies.clone())
        };

        #[cfg(target_os = "android")]
        {
            let Some(dir) =
                crate::shared::platform::android::data_dir().map(|dir| dir.join("youtube"))
            else {
                self.fail(t("No folder to save the cookies"));
                return;
            };

            if let Err(error) = std::fs::create_dir_all(&dir) {
                self.fail(error.to_string());
                return;
            }

            let path = dir.join(name);
            if let Err(error) = std::fs::write(&path, &body) {
                self.fail(error.to_string());
                return;
            }

            let mime = if sealed {
                "application/octet-stream"
            } else {
                "text/plain"
            };

            match crate::shared::platform::android::share::send(&path, mime, name) {
                Ok(()) => self.notify(t("Cookies shared")),
                Err(error) => self.fail(error),
            }

            return;
        }

        #[cfg(not(target_os = "android"))]
        {
            let Some(dir) = downloads() else {
                self.fail(t("No folder to save the cookies"));
                return;
            };

            let path = dir.join(name);
            match std::fs::write(&path, &body) {
                Ok(()) => {
                    self.notify(t("Cookies saved to {}").replace("{}", &path.display().to_string()))
                }
                Err(error) => self.fail(error.to_string()),
            }
        }
    }

    pub(crate) fn resume_spotify(&mut self, cx: &mut gpui::Context<Self>) {
        let Some(auth) = self.store.spotify.clone() else {
            return;
        };

        self.settings.update(cx, |settings, _| {
            settings.spotify = SpotifyState::Connecting
        });
        self.settings
            .update(cx, |settings, _| settings.user = auth.username.clone());

        if !auth.signed_in() {
            self.settings
                .update(cx, |settings, _| settings.spotify = SpotifyState::Failed);
            self.settings.update(cx, |settings, _| {
                settings.error = Some(t("Sign in to Spotify again").to_owned())
            });
            return;
        }

        let device = self.store.spotify_device.clone();
        self.jobs.spawn(async move {
            match spotify::Session::connect(&device, &auth).await {
                Ok(session) => Event::SpotifyResumed(session),
                Err(error) => Event::SpotifyFailed(error),
            }
        });
    }

    pub(crate) fn arm_spotify(&mut self, cx: &mut gpui::Context<Self>) {
        let backoff = self.settings.read(cx).backoff;
        self.settings.update(cx, |settings, _| {
            settings.retry = Some(Instant::now() + backoff);
            settings.backoff = (backoff * 4).min(SPOTIFY_RETRY_MAX);
        });
    }

    pub(crate) fn tick_spotify(&mut self, cx: &mut gpui::Context<Self>) -> bool {
        let Some(at) = self.settings.read(cx).retry else {
            return false;
        };

        let now = Instant::now();
        if now < at {
            return false;
        }

        self.settings
            .update(cx, |settings, _| settings.retry = None);
        if self.settings.read(cx).spotify == SpotifyState::Failed && self.store.spotify.is_some() {
            self.resume_spotify(cx);
            return true;
        }

        false
    }

    pub fn login_spotify(&mut self, cx: &mut gpui::Context<Self>) {
        if self.settings.read(cx).spotify == SpotifyState::Connecting {
            return;
        }

        self.settings.update(cx, |settings, _| {
            settings.spotify = SpotifyState::Connecting
        });
        self.notify(t("Complete the sign-in in your browser"));
        let device = self.store.spotify_device.clone();
        self.jobs
            .spawn(async move { Event::SpotifyLogin(spotify::login(device).await) });
    }

    pub fn disconnect_spotify(&mut self, cx: &mut gpui::Context<Self>) {
        self.settings
            .update(cx, |settings, _| settings.spotify = SpotifyState::Off);
        self.settings
            .update(cx, |settings, _| settings.user.clear());
        self.settings
            .update(cx, |settings, _| settings.error = None);
        self.library.update(cx, |library, _| {
            library.spotify.clear();
            library.follows.clear();
            library.friends.clear();
            library.following = Default::default();
        });
        cache::forget_library();
        // The loaded pages belong to the account that just signed out: keeping
        // them would hold its playlists, albums and artists for the session.
        self.catalog.update(cx, |catalog, _| {
            catalog.users.clear();
            catalog.failed = None;
            catalog.discography.clear();
            catalog.discography_loading.clear();
            catalog.playlists.clear();
            catalog.recent.clear();
            catalog.albums.clear();
            catalog.artists.clear();
            catalog.channels.clear();
            catalog.search.loading = false;
            catalog.search.error = None;
            catalog.pending = None;
        });
        self.library.update(cx, |library, _| {
            library.load.error = None;
            library.load.loading = false;
        });
        self.store.spotify = None;
        self.settings.update(cx, |settings, _| settings.auth = None);
        self.settings
            .update(cx, |settings, _| settings.retry = None);
        self.settings
            .update(cx, |settings, _| settings.backoff = SPOTIFY_RETRY);
        if self.ui.search_source == Source::Spotify {
            self.ui.search_source = Source::YouTube;
        }

        self.persist(cx);
    }

    pub(super) fn load_library(
        &mut self,
        session: Arc<spotify::Session>,
        _cx: &mut gpui::Context<Self>,
    ) {
        let http = self.http.clone();
        self.jobs
            .spawn(async move { Event::SpotifyLibrary(spotify::library(&http, &session).await) });
    }

    pub(crate) fn load_youtube_library(&mut self, cx: &mut gpui::Context<Self>) {
        if !self.youtube.full() {
            return;
        }

        self.settings
            .update(cx, |settings, _| settings.youtube_load += 1);
        let token = self.settings.read(cx).youtube_load;
        let youtube = self.youtube.clone();
        self.jobs
            .spawn(async move { Event::YouTubeLibrary(token, youtube.library().await) });
    }

    pub(crate) fn adopt_spotify(
        &mut self,
        session: Arc<spotify::Session>,
        cx: &mut gpui::Context<Self>,
    ) {
        self.settings
            .update(cx, |settings, _| settings.user = session.username());
        let applied = self
            .store
            .spotify
            .as_ref()
            .map(|auth| auth.data.clone())
            .unwrap_or_default();
        self.settings
            .update(cx, |settings, _| settings.applied = applied);
        self.settings
            .update(cx, |settings, _| settings.auth = Some(session.clone()));
        self.settings
            .update(cx, |settings, _| settings.spotify = SpotifyState::On);
        self.settings
            .update(cx, |settings, _| settings.error = None);
        self.settings
            .update(cx, |settings, _| settings.retry = None);
        self.settings
            .update(cx, |settings, _| settings.backoff = SPOTIFY_RETRY);
        self.library
            .update(cx, |library, _| library.load.loading = true);
        self.persist(cx);
        self.load_library(session.clone(), cx);
        self.load_following(session, cx);
        let pending = self.catalog.update(cx, |catalog, _| catalog.pending.take());
        if let Some(query) = pending {
            self.catalog
                .update(cx, |catalog, _| catalog.search.loading = false);
            if self.route == Route::Catalog(SearchType::Track, query.clone()) {
                self.ensure_catalog(&query, cx);
            } else {
                // An abandoned page runs its search again when reopened.
                self.catalog.update(cx, |catalog, _| catalog.query.clear());
            }
        }
    }

    pub fn toggle_pin(&mut self, _cx: &mut gpui::Context<Self>) {
        if !pin::pinnable() {
            self.notify(t("This session does not let a window stay on top."));
            return;
        }

        let on = !self.store.always_on_top;
        if !pin::set(on) {
            self.notify(t("This session does not let a window stay on top."));
            return;
        }

        self.store.always_on_top = on;
        self.save_store();
    }
}
