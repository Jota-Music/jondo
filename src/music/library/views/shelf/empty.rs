use gpui::{AnyElement, Context, IntoElement};

use crate::shared::i18n::t;
use crate::shared::ui::views::components::{
    hint::{self, Hint},
    loading,
};

use crate::shared::ui::views::design::icons;
use crate::shared::ui::views::design::theme;
use crate::shell::app::{Filter, Jondo, SpotifyState};

use super::Scope;

pub(super) fn empty(app: &mut Jondo, cx: &mut Context<Jondo>, scope: Scope) -> AnyElement {
    if scope == Scope::Following {
        return match app.ui.following_filter {
            Filter::All => hint::hint(
                Hint {
                    icon: icons::HEART,
                    tint: theme::rgb(0xd946ef),
                    title: t("Nothing to see yet"),
                    body: t(
                        "Follow a Spotify profile or a YouTube channel and it will show up here.",
                    ),
                    steps: &[
                        t("In the search bar at the top, search an artist, profile or channel."),
                        t("Open the profile or channel and tap the heart to follow it."),
                        t("Spotify profiles and YouTube channels you follow appear on this page."),
                    ],
                },
                None,
            )
            .into_any_element(),
            Filter::YouTube => {
                hint::plain(t("No YouTube channels followed yet.")).into_any_element()
            }
            Filter::Spotify => match app.settings.read(cx).spotify {
                SpotifyState::Off => spotify_connect(cx),
                SpotifyState::Failed => spotify_failed(app, cx),
                SpotifyState::On | SpotifyState::Connecting => {
                    hint::plain(t("No Spotify profiles followed yet.")).into_any_element()
                }
            },
            Filter::Local => hint::plain(t("Nothing to see yet")).into_any_element(),
        };
    }

    let loading = app.settings.read(cx).spotify == SpotifyState::Connecting
        || (app.library.read(cx).load.loading
            && matches!(app.ui.filter, Filter::All | Filter::Spotify));
    if loading {
        return loading::spinner().into_any_element();
    }

    match app.ui.filter {
        Filter::All => hint::hint(
            Hint {
                icon: icons::LIBRARY,
                tint: theme::MUTED,
                title: t("No playlists yet"),
                body: t("Connect Spotify, follow a YouTube playlist, or create a local one — everything shows up here."),
                steps: &[
                    t("In the search bar at the top, search a playlist by name or paste its link."),
                    t("Open the Playlists tab in the results and tap the heart to follow it."),
                    t("Or use the + button to create a local playlist."),
                ],
            },
            None,
        )
        .into_any_element(),
        Filter::YouTube => hint::hint(
            Hint {
                icon: icons::Icon::YouTube,
                tint: theme::rgb(0xef4444),
                title: t("No playlists yet"),
                body: t("Follow a YouTube playlist or create a local one — both show up here."),
                steps: &[
                    t("In the search bar at the top, search a playlist by name or paste its link."),
                    t("Open the Playlists tab in the results and tap the heart to follow it."),
                    t("Or use the + button to create a local playlist."),
                ],
            },
            None,
        )
        .into_any_element(),
        Filter::Local => hint::hint(
            Hint {
                icon: icons::LIBRARY,
                tint: theme::rgb(0x10b981),
                title: t("No local playlists yet"),
                body: t("Create a local playlist to collect songs that live on this device."),
                steps: &[
                    t("Tap the + button in the top corner of your playlists."),
                    t("Name it and it appears in this list."),
                    t("Open it and add songs from any source."),
                ],
            },
            Some(hint::action(
                "local-create",
                t("New playlist"),
                Some(icons::PLUS),
                cx.listener(|app, _: &gpui::ClickEvent, window, cx| {
                    app.ui.create_open = true;
                    app.set_field("create", "", window, cx);
                    cx.notify();
                }),
            )),
        )
        .into_any_element(),
        Filter::Spotify => match app.settings.read(cx).spotify {
            SpotifyState::Off => spotify_connect(cx),
            SpotifyState::Failed => spotify_failed(app, cx),
            SpotifyState::On | SpotifyState::Connecting => {
                hint::plain(t("No Spotify playlists.")).into_any_element()
            }
        },
    }
}

fn spotify_connect(cx: &mut Context<Jondo>) -> AnyElement {
    hint::hint(
        Hint {
            icon: icons::Icon::Spotify,
            tint: theme::SPOTIFY,
            title: t("Connect Spotify"),
            body: t("Risk: Spotify may block an account that uses Jondo."),
            steps: &[
                t("Tap Connect with Spotify and log in with your account."),
                t("Return to Jondo when the login finishes."),
                t("Your playlists and followed profiles appear on their own."),
            ],
        },
        Some(hint::action(
            "spotify-connect",
            t("Connect with Spotify"),
            Some(icons::Icon::Spotify),
            cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                app.login_spotify(cx);
                cx.notify();
            }),
        )),
    )
    .into_any_element()
}

fn spotify_failed(app: &mut Jondo, cx: &mut Context<Jondo>) -> AnyElement {
    let error = app.settings.read(cx).error.clone();
    hint::hint(
        Hint {
            icon: icons::CIRCLE_ALERT,
            tint: theme::AMBER,
            title: t("Spotify is unreachable"),
            body: error.as_deref().unwrap_or(t(
                "This needs a live connection. Your saved library still works.",
            )),
            steps: &[],
        },
        Some(hint::action(
            "spotify-retry",
            t("Retry"),
            None,
            cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                app.login_spotify(cx);
                cx.notify();
            }),
        )),
    )
    .into_any_element()
}
