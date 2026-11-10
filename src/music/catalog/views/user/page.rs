use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Window, div,
};

use crate::music::library::views::playlist;
use crate::shared::i18n::t;
use crate::shared::music::domain::{Follow, FollowKind};
use crate::shared::music::views::cover;
use crate::shared::ui::views::components::{grid, hint};
use crate::shared::ui::views::design::icons;
use crate::shared::ui::views::design::theme;
use crate::shell::app::{Jondo, Route};
use crate::shell::views::page;

/// The profile before its data lands: the avatar and the name lines shimmer in
/// the header, and the grid keeps the cards' frame.
fn skeleton(app: &mut Jondo, window: &Window, username: &str) -> AnyElement {
    let header = page::pending(64.0, 32.0)
        .child(
            div()
                .w(theme::unit(32.0))
                .h(theme::unit(32.0))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .rounded(theme::unit(6.0))
                .border_1()
                .border_color(theme::ZINC_800)
                .bg(theme::SURFACE)
                .child(cover::brand_mono(
                    crate::shared::music::domain::Source::YouTube,
                    16.0,
                    theme::MUTED,
                )),
        )
        .child(
            div()
                .w(theme::unit(32.0))
                .h(theme::unit(32.0))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .rounded(theme::unit(6.0))
                .border_1()
                .border_color(theme::ZINC_800)
                .bg(theme::SURFACE)
                .child(icons::icon(icons::HEART, 14.0, theme::ZINC_700)),
        );
    let cell = grid::cell(window);
    let art = cell - 12.0;
    let tiles = (0..6)
        .map(|_| {
            div()
                .w(theme::unit(cell))
                .flex_none()
                .flex()
                .flex_col()
                .child(
                    div()
                        .px(theme::unit(6.0))
                        .pt(theme::unit(6.0))
                        .child(cover::shimmer(art, art, 6.0)),
                )
                .child(
                    div()
                        .px(theme::unit(6.0))
                        .pt(theme::unit(8.0))
                        .pb(theme::unit(6.0))
                        .flex()
                        .flex_col()
                        .gap(theme::unit(2.0))
                        .child(cover::shimmer(art * 0.7, 14.0, 4.0))
                        .child(cover::shimmer(art * 0.45, 12.0, 4.0)),
                )
                .into_any_element()
        })
        .collect();
    let scroll = app.pane(format!("user-playlists:{username}"));
    let content = page::panel(grid::show("user-playlists-loading", &scroll, window, tiles));
    div()
        .size_full()
        .flex()
        .flex_col()
        .gap(theme::unit(24.0))
        .child(header)
        .child(content)
        .into_any_element()
}

pub fn show(app: &mut Jondo, window: &mut Window, cx: &mut Context<Jondo>) -> AnyElement {
    let Route::User(username) = app.route.clone() else {
        return div().into_any_element();
    };

    let Some(user) = app.user(&username, cx) else {
        if app.catalog.read(cx).users.loading(&username) {
            return skeleton(app, window, &username);
        }

        return hint::plain(t("User not found")).into_any_element();
    };

    let user_name = user.name.clone();
    let user_image = user.image.clone();
    let playlists = app.user_playlists(&username, cx);
    let grid = app.ui.user_grid;
    let follow = Follow::spotify(FollowKind::User, &username, &user_name, user_image.clone());
    let extra = div()
        .id("user-youtube")
        .w(theme::unit(32.0))
        .h(theme::unit(32.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(theme::unit(6.0))
        .border_1()
        .border_color(theme::ZINC_800)
        .bg(theme::SURFACE)
        .cursor_pointer()
        .on_click(cx.listener({
            let name = user_name.clone();
            move |app, _: &gpui::ClickEvent, window, cx| {
                app.set_field("search", &name, window, cx);
                app.search(name.clone(), cx);
                cx.notify();
            }
        }))
        .child(cover::brand_mono(
            crate::shared::music::domain::Source::YouTube,
            16.0,
            theme::MUTED,
        ))
        .into_any_element();
    let header = page::show(
        app,
        cx,
        page::Header {
            cover: user_image.as_deref(),
            covers: &[],
            radius: 32.0,
            title: &user_name,
            subtitle: Some(format!("@{username}")),
            link: None,
            save: None,
            follow: Some(follow),
            play: None,
            extra: Some(extra),
            actions: None,
        },
    );

    let controls = div()
        .w_full()
        .flex()
        .items_center()
        .justify_end()
        .child(grid::toggle(
            "user-view",
            grid,
            32.0,
            cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                app.ui.user_grid = true;
                cx.notify();
            }),
            cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                app.ui.user_grid = false;
                cx.notify();
            }),
        ));

    let content = if playlists.is_empty() {
        page::panel(hint::muted(t("No playlists.")).into_any_element())
    } else {
        page::panel(playlist::results(
            app,
            window,
            cx,
            format!("user-playlists:{username}"),
            "user-playlists",
            playlists,
            grid,
        ))
    };

    div()
        .size_full()
        .flex()
        .flex_col()
        .gap(theme::unit(16.0))
        .child(header)
        .child(
            div()
                .flex_1()
                .min_h(theme::unit(0.0))
                .flex()
                .flex_col()
                .gap(theme::unit(8.0))
                .child(controls)
                .child(content),
        )
        .into_any_element()
}

#[cfg(test)]
#[path = "page.test.rs"]
mod tests;
