use gpui::{
    AnyElement, Context, ParentElement, Role, StatefulInteractiveElement, Styled, Window, div,
};

use gpui_kit::component::input::Textarea;
use gpui_kit::component::{Sizable as _, Size};

use crate::shared::i18n::t;
use crate::shared::ui::views::components::{hover, text};
use crate::shared::ui::views::design::icons::{self, Icon};
use crate::shared::ui::views::design::theme;
use crate::shell::app::{Jondo, SpotifyState};

use super::super::chrome::{card, danger_button, filled_button, section};

use super::pieces::{browser_import, paragraph, spotify_card};

pub(crate) fn account(app: &mut Jondo, window: &mut Window, cx: &mut Context<Jondo>) -> AnyElement {
    let spotify = app.settings.read(cx).spotify == SpotifyState::On;
    let spotify_user = app.settings.read(cx).user.clone();
    let youtube = app.youtube.signed_in();
    let full = app.store.youtube_full;
    let open = app.ui.cookies_open;
    let importing = app.ui.cookies_importing;
    let picking = app
        .menu
        .as_ref()
        .is_some_and(|menu| menu.origin == Some("settings-browser"));
    let error = app.ui.cookies_error.clone();

    let browsers = app.settings.read(cx).browsers.clone();
    let browser = browsers
        .iter()
        .copied()
        .find(|name| *name == app.ui.browser_pick.as_str())
        .or_else(|| browsers.first().copied())
        .unwrap_or("");

    let mut body = div().flex().flex_col().gap(theme::unit(16.0));
    body = body.child(spotify_card(cx, spotify, spotify_user));
    let youtube_body = youtube_body(
        app, window, cx, youtube, full, &browsers, browser, importing, picking, open, &error,
    );
    section(
        Icon::UserRound,
        t("Account"),
        None,
        body.child(youtube_body),
    )
}

#[allow(clippy::too_many_arguments)]
fn youtube_body(
    app: &mut Jondo,
    window: &mut Window,
    cx: &mut Context<Jondo>,
    youtube: bool,
    full: bool,
    browsers: &[&'static str],
    browser: &'static str,
    importing: bool,
    picking: bool,
    open: bool,
    error: &Option<String>,
) -> gpui::Div {
    let mut body = div()
        .flex()
        .flex_col()
        .gap(theme::unit(8.0))
        .border_t_1()
        .border_color(theme::ZINC_800)
        .pt(theme::unit(16.0))
        .child(text::text(
            t("YouTube — needed for age-restricted videos and your own playlists"),
            12.0,
            theme::FAINT,
        ));
    if youtube {
        body = body.child(connected(
            cx, full, browsers, browser, importing, picking, error,
        ));
    } else {
        body = body.child(cookies(
            app, window, cx, browsers, browser, importing, picking, open, error,
        ));
    }

    body
}

fn connected(
    cx: &mut Context<Jondo>,
    full: bool,
    browsers: &[&'static str],
    browser: &'static str,
    importing: bool,
    picking: bool,
    error: &Option<String>,
) -> gpui::Div {
    let mut body = div().flex().flex_col().gap(theme::unit(8.0));
    body = body.child(card(
        div()
            .flex()
            .items_center()
            .justify_between()
            .gap(theme::unit(12.0))
            .child(
                div()
                    .min_w(theme::unit(0.0))
                    .flex()
                    .flex_col()
                    .child(text::text(t("Connected as"), 12.0, theme::FAINT))
                    .child(text::text(t("Signed in"), 14.0, theme::ZINC_100)),
            )
            .child(danger_button(
                "youtube-signout",
                t("Sign out"),
                cx.listener(|app, _: &gpui::ClickEvent, window, cx| {
                    app.sign_out_youtube(cx);
                    if let Some(area) = app.textareas.get("cookies").cloned() {
                        area.update(cx, |state, cx| state.set_value("", window, cx));
                    }

                    cx.notify();
                }),
            )),
    ));

    if !full {
        let mut partial = div()
            .rounded(theme::unit(12.0))
            .border_1()
            .border_color(theme::alpha(theme::AMBER_500, 0.4))
            .bg(theme::alpha(theme::AMBER_500, 0.1))
            .p(theme::unit(12.0))
            .flex()
            .flex_col()
            .gap(theme::unit(12.0))
            .child(text::text(
                t("This session cannot open private playlists. Use the session from a browser:"),
                12.0,
                theme::AMBER_300,
            ));
        if !browsers.is_empty() {
            partial = partial.child(browser_import(browsers, browser, importing, picking, cx));
        }

        if let Some(message) = &error {
            partial = partial.child(text::text(message.clone(), 12.0, theme::DANGER));
        }

        body = body.child(partial);
    }

    body = body.child(filled_button(
        "youtube-export",
        t("Export cookies"),
        false,
        true,
        true,
        false,
        cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
            app.export_cookies(cx);
            cx.notify();
        }),
    ));
    body
}

#[allow(clippy::too_many_arguments)]
fn cookies(
    app: &mut Jondo,
    window: &mut Window,
    cx: &mut Context<Jondo>,
    browsers: &[&'static str],
    browser: &'static str,
    importing: bool,
    picking: bool,
    open: bool,
    error: &Option<String>,
) -> gpui::Div {
    let mut body = div();
    let area = app.cookies_area(window, cx);
    let mut panel = div().flex().flex_col().gap(theme::unit(16.0));
    if !browsers.is_empty() {
        panel = panel.child(browser_import(browsers, browser, importing, picking, cx));
    }

    #[cfg(target_os = "android")]
    {
        panel = panel.child(paragraph(
                t("In-app sign-in is not available on this platform. Paste your cookies or import a cookies.txt export below."),
                theme::FAINT,
            ));
    }

    let save_area = area.clone();
    let save = cx.listener(move |app, _: &gpui::ClickEvent, window, cx| {
        if app.ui.cookies_draft.trim().is_empty() {
            return;
        }

        app.sign_in_youtube(cx);
        let draft = app.ui.cookies_draft.clone();
        save_area.update(cx, |state, cx| state.set_value(draft, window, cx));
        cx.notify();
    });

    #[allow(unused_mut)]
    let mut actions = div()
        .flex()
        .items_center()
        .gap(theme::unit(8.0))
        .child(filled_button(
            "cookies-save",
            t("Save"),
            false,
            !app.ui.cookies_draft.trim().is_empty(),
            false,
            false,
            save,
        ));

    #[cfg(not(target_os = "android"))]
    {
        actions = actions.child(hover::hover(
            "cookies-import",
            true,
            move |fade| {
                div()
                    .h(theme::unit(32.0))
                    .px(theme::unit(12.0))
                    .flex()
                    .items_center()
                    .child(text::weighted(
                        t("Import file"),
                        12.0,
                        theme::mix(theme::MUTED, theme::ZINC_200, fade),
                        theme::Weight::SemiBold,
                    ))
            },
            cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                app.import_cookies_file(cx);
                cx.notify();
            }),
        ));
    }

    let mut details = div()
        .rounded(theme::unit(12.0))
        .border_1()
        .border_color(theme::ZINC_800)
        .bg(theme::SURFACE)
        .overflow_hidden()
        .child(
            hover::hover(
                "cookies-toggle",
                true,
                move |fade| {
                    let color = theme::mix(theme::MUTED, theme::ZINC_200, fade);
                    div()
                        .p(theme::unit(12.0))
                        .flex()
                        .items_center()
                        .gap(theme::unit(8.0))
                        .child(icons::icon(
                            if open {
                                Icon::ChevronDown
                            } else {
                                Icon::ChevronRight
                            },
                            14.0,
                            color,
                        ))
                        .child(
                            text::weighted(
                                t("Paste cookies or import a cookies.txt"),
                                12.0,
                                color,
                                theme::Weight::SemiBold,
                            )
                            .flex_1()
                            .min_w(theme::unit(0.0)),
                        )
                },
                cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                    app.ui.cookies_open = !app.ui.cookies_open;
                    cx.notify();
                }),
            )
            .role(Role::Button)
            .aria_expanded(open)
            .aria_label(t("Paste cookies or import a cookies.txt")),
        );

    if open {
        details = details.child(
                div()
                    .border_t_1()
                    .border_color(theme::ZINC_800)
                    .p(theme::unit(12.0))
                    .flex()
                    .flex_col()
                    .gap(theme::unit(12.0))
                    .child(paragraph(
                        t("Import a cookies.txt exported while signed in: on a youtube.com tab use an extension (Get cookies.txt), or run copy(document.cookie) only if you do not need private playlists. The cookies that open your lists (__Secure-3PSID) are HttpOnly and are not in document.cookie."),
                        theme::FAINT,
                    ))
                    .child(
                        div()
                            .rounded(theme::unit(8.0))
                            .border_1()
                            .border_color(theme::ZINC_800)
                            .bg(theme::SURFACE)
                            .px(theme::unit(8.0))
                            .py(theme::unit(12.0))
                            .child(
                                Textarea::new(&area)
                                    .appearance(false)
                                    .bordered(false)
                                    .with_size(Size::XSmall)
                                    .h(theme::unit(72.0))
                                    .w_full()
                                    .text_size(theme::unit(12.0))
                                    .text_color(theme::ZINC_200),
                            ),
                    )
                    .child(actions),
            );
    }

    panel = panel.child(details);
    panel = panel.child(
            div()
                .flex()
                .gap(theme::unit(8.0))
                .child(icons::icon(
                    Icon::ShieldAlert,
                    14.0,
                    theme::alpha(theme::AMBER_300, 0.8),
                ))
                .child(paragraph(
                    t("Signing in with a Google account and using it to play audio can get that account banned by Google. Use an account you can afford to lose, and prefer a secondary one."),
                    theme::alpha(theme::AMBER_300, 0.8),
                ).flex_1().min_w(theme::unit(0.0))),
        );

    if let Some(message) = &error {
        panel = panel.child(text::text(message.clone(), 12.0, theme::DANGER));
    }

    body = body.child(card(panel).p(theme::unit(16.0)));
    body
}
