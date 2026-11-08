use gpui::prelude::FluentBuilder as _;
use gpui::{
    Animation, AnimationExt, AnyElement, Context, InteractiveElement, IntoElement, ParentElement,
    Styled, Window, div,
};

use crate::music::catalog::views as catalog;
use crate::music::catalog::views::user;
use crate::music::library::views::{following, playlist, playlists, shelf};
use crate::music::player::views as player;
use crate::music::rooms::views as rooms;
use crate::music::track::views as track;
use crate::settings::views;
use crate::shared::ui::views::components::page;
use crate::shared::ui::views::design::theme;
use crate::shell::app::{Jondo, Route};
use crate::shell::views::clipboard;
use crate::shell::views::header;
use crate::shell::views::menu;
use crate::shell::views::nav;
use crate::shell::views::update;

use super::chrome::{bottom, resize_bands, safe_area};
use super::toast::toast;

pub fn show(app: &mut Jondo, window: &mut Window, cx: &mut Context<Jondo>) -> impl IntoElement {
    let narrow = window.viewport_size().width < theme::unit(768.0);
    let route = app.route_key.clone();
    let screen = div()
        .size_full()
        .child(screen(app, window, cx))
        .with_animation(
            route,
            Animation::new(std::time::Duration::from_millis(140)),
            |screen, delta| screen.opacity(delta),
        )
        .into_any_element();

    let has_song = app.player.read(cx).current().is_some();
    let compact = app.ui.compact && has_song;
    let mut root = div()
        .relative()
        .flex()
        .flex_col()
        .bg(theme::BG)
        .on_mouse_up(
            gpui::MouseButton::Left,
            cx.listener(|app, _: &gpui::MouseUpEvent, _window, cx| {
                app.finish_drag(cx);
                cx.notify();
            }),
        )
        .child(header::show(app, window, cx));

    root = root.child(update::show(app, cx));
    root = root.child(clipboard::show(app, cx));
    root = root.child(track::selection::bar(app, cx));

    if !narrow && !compact && has_song {
        root = root.child(player::panel(app, window, cx));
    }

    root = root.child(
        div()
            .flex_1()
            .min_h(theme::unit(0.0))
            .w_full()
            .overflow_hidden()
            .py(theme::unit(24.0))
            .when(narrow, |content| content.px(theme::unit(16.0)))
            .child(page(screen)),
    );

    // Overlays live in the area between the header and the phone navigation,
    // so a sheet's bottom edge lands on the nav instead of sliding under it.
    let (inset_top, inset_bottom, _, _) = safe_area();
    root = root.child(
        div()
            .absolute()
            .left_0()
            .right_0()
            .top(theme::unit(inset_top + theme::HEADER_HEIGHT))
            .bottom(theme::unit(
                inset_bottom + if narrow { theme::NAV_HEIGHT } else { 0.0 },
            ))
            .child(player::overlay(app, window, cx))
            .child(player::queue(app, window, cx))
            .child(playlists::show(app, window, cx))
            .child(views::overlay(app, window, cx))
            .child(rooms::overlay(app, window, cx)),
    );

    // After the overlays so the bar keeps its colour and its clicks while a
    // sheet's scrim is up, like the web bar over the modal backdrop. On a
    // phone the menu belongs to that same layer, so its sheet slides out from
    // under the player instead of covering it; popovers stay above.
    let popover = app.ui.menu.as_ref().is_some_and(|menu| menu.context);
    let menu = menu::show(app, window, cx);
    let bar = (has_song && (narrow || compact)).then(|| player::bar(app, window, cx));
    root = if narrow && !popover {
        root.child(menu).children(bar)
    } else {
        root.children(bar).child(menu)
    };

    root = root.child(toast(app, narrow)).child({
        let nav = nav::nav(app, cx, narrow);
        if narrow {
            div()
                .absolute()
                .left_0()
                .right_0()
                .bottom(theme::unit(safe_area().1))
                .child(nav)
                .into_any_element()
        } else {
            nav
        }
    });

    if !window.is_maximized() && cfg!(not(target_os = "android")) {
        root = root.child(resize_bands(cx));
    }

    fit(root, window, narrow)
}

/// A phone has no fixed viewport: the status bar, the gesture bar and the
/// keyboard each take a slice. The layout follows what is actually visible,
/// which is the native counterpart of the web's `100dvh` and safe-area insets.
fn fit(root: gpui::Div, window: &Window, narrow: bool) -> gpui::Div {
    let (top, _, left, right) = safe_area();
    root.size_full()
        .pt(theme::unit(top))
        .pb(theme::unit(bottom(window, narrow)))
        .pl(theme::unit(left))
        .pr(theme::unit(right))
}

fn screen(app: &mut Jondo, window: &mut Window, cx: &mut Context<Jondo>) -> AnyElement {
    match app.route.clone() {
        Route::Home => shelf::show(app, window, cx).into_any_element(),
        Route::Playlist(_) => playlist::show(app, window, cx),
        Route::Search(_) => track::search::show(app, window, cx),
        Route::Settings => views::show(app, window, cx),
        Route::Following => following::show(app, window, cx),
        Route::Album(_) => user::album::show(app, cx),
        Route::Artist(_) => user::artist::show(app, window, cx),
        Route::Channel(_) => user::channel::show(app, window, cx),
        Route::User(_) => user::show(app, window, cx),
        Route::Catalog(_, _) => catalog::show(app, cx),
        Route::Rooms => rooms::show(app, window, cx),
    }
}
