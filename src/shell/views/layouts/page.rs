use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement, StyleRefinement, Styled,
    Window, div,
};

use crate::music::catalog::views as catalog;
use crate::music::catalog::views::user;
use crate::music::library::views::{following, playlist, playlists, shelf};
use crate::music::player::views as player;
use crate::music::rooms::views as rooms;
use crate::music::track::views as track;
use crate::settings::views;
use crate::shared::ui::views::components::{hover, page};
use crate::shared::ui::views::design::theme;
use crate::shell::app::{Jondo, Route, motion};
use crate::shell::views::clipboard;
use crate::shell::views::header;
use crate::shell::views::menu;
use crate::shell::views::nav;
use crate::shell::views::update;

use super::chrome::{bottom, resize_bands, safe_area, system};
use super::toast::toast;

/// How long the page takes to appear after a route change.
const FADE: std::time::Duration = std::time::Duration::from_millis(140);

pub fn show(app: &mut Jondo, window: &mut Window, cx: &mut Context<Jondo>) -> impl IntoElement {
    let narrow = window.viewport_size().width < theme::unit(768.0);
    // The page is a view of its own, so a moving overlay does not rebuild it:
    // it repaints for a page state change or for a frame that is not just
    // motion, and a page hover that only exists there.
    let moving = motion::took();
    let hovering = hover::page_running();
    if paints_page(moving, hovering) {
        app.paint_page(cx);
    }
    let screen = app.screen.clone();

    // The page fades in from the background instead of animating its own
    // opacity: an opacity on the page invalidates the view cache every frame,
    // so the fade would rebuild and lay out the whole screen 8 times.
    let fade = app
        .route_changed_at
        .map(|at| 1.0 - (at.elapsed().as_secs_f32() / FADE.as_secs_f32()).clamp(0.0, 1.0));
    if fade.is_some_and(|fade| fade > 0.0) {
        motion::ask(window);
    }

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

    let panel = !narrow && !compact && has_song;
    if panel {
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
            .relative()
            .child(page(screen.cached(StyleRefinement::default().size_full())))
            .when_some(fade.filter(|fade| *fade > 0.0), |content, fade| {
                content.child(div().absolute().inset_0().bg(theme::alpha(theme::BG, fade)))
            }),
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
            .overflow_hidden()
            .child(player::overlay(app, window, cx))
            .child(player::queue(app, window, cx))
            .child(playlists::show(app, window, cx))
            .child(views::overlay(app, window, cx))
            .child(rooms::overlay(app, window, cx)),
    );

    // After the overlays so the bar keeps its colour and its clicks while a
    // sheet's scrim is up, like the web bar over the modal backdrop. On a
    // phone the player and the navigation always sit above the menu, including
    // a context menu opened by a long press, so neither is ever covered.
    let menu = menu::show(app, window, cx);
    let bar = (!panel).then(|| player::bar(app, window, cx));
    root = if narrow {
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

    // Android 15 draws edge to edge and ignores the navigation bar colour, so
    // the strip under the system buttons is whatever the app paints there. Paint
    // it last: a sheet sliding out otherwise shows through the transparent bar.
    if narrow {
        root = root.child(
            div()
                .absolute()
                .left_0()
                .right_0()
                .bottom_0()
                .h(theme::unit(system(window)))
                .bg(theme::BG),
        );
    }

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

/// Whether the page repaints this frame: a frame that is not only motion, or a
/// hover fade that lives in the page.
fn paints_page(moving: bool, hovering: bool) -> bool {
    !moving || hovering
}

pub(super) fn route(app: &mut Jondo, window: &mut Window, cx: &mut Context<Jondo>) -> AnyElement {
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

#[cfg(test)]
mod tests {
    use super::paints_page;

    #[test]
    fn a_page_hover_repaints_a_motion_frame() {
        assert!(paints_page(true, true));
        assert!(!paints_page(true, false));
        assert!(paints_page(false, false));
    }
}
