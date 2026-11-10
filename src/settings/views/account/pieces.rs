use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement, Role,
    StatefulInteractiveElement, Styled, div,
};

use crate::shared::i18n::t;
use crate::shared::ui::views::components::text;
use crate::shared::ui::views::design::icons::{self};
use crate::shared::ui::views::design::theme;
use crate::shell::app::Jondo;
use crate::shell::views::menu::{Command, Item, Menu};

use super::super::chrome::{card, danger_button, filled_button};

pub(super) fn paragraph(value: impl Into<gpui::SharedString>, color: gpui::Rgba) -> gpui::Div {
    text::text(value, 12.0, color).line_height(gpui::relative(1.625))
}

pub(super) fn browser_import(
    browsers: &[&'static str],
    browser: &'static str,
    importing: bool,
    picking: bool,
    cx: &mut Context<Jondo>,
) -> AnyElement {
    let mut body = div().flex().flex_col().gap(theme::unit(12.0));
    if browsers.len() > 1 {
        body = body.child(
            div()
                .id("settings-browser")
                .h(theme::unit(40.0))
                .w_full()
                .px(theme::unit(12.0))
                .flex()
                .items_center()
                .justify_between()
                .rounded(theme::unit(8.0))
                .border_1()
                .border_color(if picking {
                    theme::ZINC_600
                } else {
                    theme::ZINC_800
                })
                .bg(theme::SURFACE)
                .cursor_pointer()
                .role(Role::Button)
                .aria_expanded(picking)
                .aria_label(format!("{}: {}", t("Browser"), browser))
                .on_click(cx.listener(move |app, event: &gpui::ClickEvent, _, cx| {
                    let items = app
                        .settings
                        .read(cx)
                        .browsers
                        .iter()
                        .map(|name| {
                            Item::choice(
                                *name,
                                *name == browser,
                                Command::Browser((*name).to_owned()),
                            )
                        })
                        .collect();
                    app.menu = Some(
                        Menu::choice(event.position(), t("Browser"), items)
                            .origin("settings-browser"),
                    );
                    app.notify_shell(cx);
                }))
                .child(text::text(browser, 14.0, theme::ZINC_100))
                .child(icons::icon(icons::CHEVRON_DOWN, 16.0, theme::MUTED)),
        );
    } else {
        body = body.child(text::text(browser, 12.0, theme::FAINT));
    }

    let label = if importing {
        t("Reading the cookies of {}…").replace("{}", browser)
    } else {
        t("Use the {} session").replace("{}", browser)
    };

    body = body.child(filled_button(
        "youtube-browser",
        label,
        importing,
        !importing,
        true,
        true,
        cx.listener(move |app, _: &gpui::ClickEvent, _, cx| {
            app.sign_in_browser(browser, cx);
            cx.notify();
        }),
    ));
    body = body.child(text::text(
        t("Reads the YouTube session {} already holds on this machine. Close it first if the read fails.")
            .replace("{}", browser),
        12.0,
        theme::FAINT,
    ));
    body.into_any_element()
}

pub(super) fn spotify_card(
    cx: &mut Context<Jondo>,
    spotify: bool,
    spotify_user: String,
) -> gpui::Div {
    let mut body = div();
    if spotify {
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
                        .child(text::truncated(
                            spotify_user,
                            14.0,
                            theme::ZINC_100,
                            theme::Weight::Regular,
                        )),
                )
                .child(danger_button(
                    "spotify-disconnect",
                    t("Disconnect"),
                    cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                        app.disconnect_spotify(cx);
                        cx.notify();
                    }),
                )),
        ));
    } else {
        body = body.child(card(
            div()
                .p(theme::unit(12.0))
                .flex()
                .flex_col()
                .items_center()
                .gap(theme::unit(10.0))
                .child(crate::shared::music::views::cover::brand(
                    crate::shared::music::domain::Source::Spotify,
                    24.0,
                ))
                .child(text::text(
                    t("Connect with Spotify to use this."),
                    14.0,
                    theme::MUTED,
                ))
                .child(
                    div()
                        .id("spotify-connect")
                        .h(theme::unit(32.0))
                        .px(theme::unit(12.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(theme::unit(8.0))
                        .bg(theme::SPOTIFY)
                        .cursor_pointer()
                        .on_click(cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                            app.login_spotify(cx);
                            cx.notify();
                        }))
                        .child(text::weighted(
                            t("Connect with Spotify"),
                            12.0,
                            theme::BINARY,
                            theme::Weight::SemiBold,
                        )),
                ),
        ));
    }

    body
}
