use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, App, Context, Div, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Window, div,
};

use crate::music::track::views::actions as track_actions;
use crate::shared::music::domain::Playlist;
use crate::shared::music::views::cover;
use crate::shared::ui::views::components::{button, text};
use crate::shared::ui::views::design::icons;
use crate::shared::ui::views::design::theme;
use crate::shell::app::{Jondo, Route};
use crate::shell::views::menu::Item;

pub type Click = Box<dyn Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static>;
pub type Actions = Box<dyn Fn(&Jondo, &App) -> Vec<Item>>;

pub struct Play {
    pub enabled: bool,
    pub playing: bool,
    pub loading: bool,
    pub on_click: Click,
}

pub struct Header<'a> {
    pub cover: Option<&'a str>,
    pub covers: &'a [String],
    pub radius: f32,
    pub title: &'a str,
    pub subtitle: Option<String>,
    pub link: Option<(String, Route)>,
    pub save: Option<Playlist>,
    pub follow: Option<crate::shared::music::domain::Follow>,
    pub play: Option<Play>,
    pub extra: Option<AnyElement>,
    pub actions: Option<Actions>,
}

pub fn show(app: &mut Jondo, cx: &mut Context<Jondo>, header: Header<'_>) -> AnyElement {
    let art = cover::playlist(
        &mut app.covers,
        header.cover,
        header.covers,
        64.0,
        64.0,
        header.radius,
    );

    let play = header.play.map(|play| {
        let fill = if play.enabled {
            app.accent
        } else {
            theme::alpha(app.accent, 0.7)
        };

        let color = app.binary;
        if play.loading {
            button::circle_spinner(40.0, 20.0, fill, color).into_any_element()
        } else {
            button::circle(
                "page-play",
                if play.playing {
                    icons::PAUSE
                } else {
                    icons::PLAY
                },
                40.0,
                20.0,
                fill,
                color,
                play.enabled,
                play.on_click,
            )
            .into_any_element()
        }
    });

    let mut actions = div().flex().items_center().gap(theme::unit(8.0));
    if let Some(extra) = header.extra {
        actions = actions.child(extra);
    }

    if let Some(save) = header.save {
        let on = app.is_saved(&save.id);
        actions = actions.child(button::heart(
            "page-save",
            on,
            cx.listener(move |app, _: &gpui::ClickEvent, _, cx| {
                app.toggle_saved(&save, cx);
                cx.notify();
            }),
        ));
    }

    if let Some(follow) = header.follow {
        let on = app.following().iter().any(|artist| artist.id == follow.id);
        actions = actions.child(button::heart(
            "page-follow",
            on,
            cx.listener(move |app, _: &gpui::ClickEvent, _, cx| {
                app.toggle_follow(&follow, cx);
                cx.notify();
            }),
        ));
    }

    if let Some(play) = play {
        actions = actions.child(play);
    }

    if let Some(build) = header.actions {
        actions = actions.child(track_actions::trigger(
            cx,
            "page-more",
            move |app, cx| (None, build(app, cx)),
            0.0,
        ));
    }

    div()
        .flex()
        .items_center()
        .gap(theme::unit(16.0))
        .child(art)
        .child(
            div()
                .flex_1()
                .min_w(theme::unit(120.0))
                .flex()
                .flex_col()
                .gap(theme::unit(4.0))
                .child(text::truncated(
                    header.title.to_owned(),
                    20.0,
                    theme::TEXT,
                    theme::Weight::SemiBold,
                ))
                .when_some(header.subtitle, |this, subtitle| {
                    this.child(text::truncated(
                        subtitle,
                        14.0,
                        theme::white(0.7),
                        theme::Weight::Regular,
                    ))
                })
                .when_some(header.link, |this, (label, route)| {
                    this.child(
                        div()
                            .id(format!("page-link-{label}"))
                            .cursor_pointer()
                            .hover(|style| style.underline())
                            .on_click(cx.listener(move |app, _: &gpui::ClickEvent, _, cx| {
                                app.open(route.clone(), cx);
                                cx.notify();
                            }))
                            .child(text::truncated(
                                label,
                                14.0,
                                theme::white(0.7),
                                theme::Weight::Regular,
                            )),
                    )
                }),
        )
        .child(actions)
        .into_any_element()
}

/// The header a page paints while its data is still on its way: the artwork
/// box and the name lines shimmer where the real header will land, so a page
/// never starts with a spinner and then swaps a whole header in. The caller
/// appends the actions it will keep once the data arrives.
pub fn pending(size: f32, radius: f32) -> Div {
    div()
        .flex()
        .items_center()
        .gap(theme::unit(16.0))
        .child(cover::shimmer(size, size, radius))
        .child(
            div()
                .flex_1()
                .min_w(theme::unit(120.0))
                .flex()
                .flex_col()
                .gap(theme::unit(8.0))
                .overflow_hidden()
                .child(cover::shimmer(180.0, 20.0, 4.0))
                .child(cover::shimmer(110.0, 14.0, 4.0)),
        )
}

/// A page that is still loading: the same centred spinner every other view
/// uses, so no page invents its own loading treatment.
pub fn loading() -> AnyElement {
    div()
        .size_full()
        .flex()
        .child(crate::shared::ui::views::components::loading::spinner())
        .into_any_element()
}

pub fn panel(inner: impl IntoElement) -> AnyElement {
    crate::shared::ui::views::components::panel(inner).into_any_element()
}
