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
use crate::shell::views::sheet;
use crate::shell::views::update;

use super::chrome::{bottom, resize_bands, safe_area, system};
use super::toast::toast;

pub fn show(app: &mut Jondo, window: &mut Window, cx: &mut Context<Jondo>) -> impl IntoElement {
    let narrow = window.viewport_size().width < theme::unit(768.0);
    // Panels and page heads deep in the tree bleed their frame on a phone, and
    // none of them carries the window: the shell publishes the side of the
    // breakpoint once per paint instead.
    crate::shared::ui::views::components::set_narrow(narrow);
    // The cover slots and the scrollbar thumbs choose their tone from this.
    crate::shared::ui::views::components::set_ambient(app.store.backdrop);
    // The page is a view of its own, so a moving overlay does not rebuild it:
    // it repaints for a page state change or for a frame that is not just
    // motion, and a page hover that only exists there.
    let moving = motion::took();
    let hovering = hover::page_running();
    if paints_page(moving, hovering) {
        app.paint_page(cx);
    }
    let screen = app.screen.clone();

    let has_song = app.player.read(cx).current().is_some();
    let compact = app.ui.compact && has_song;
    // The kit's root already fills the window with the same `theme::BG`, so
    // painting it again here only costs a full-screen pass on the GPU.
    let mut root = div()
        .relative()
        .flex()
        .flex_col()
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

    // The glow reaches the desktop player and the page: they are the same band
    // of the shell, the one between the header and the phone navigation. It
    // used to sit inside the page alone, which left the player on black.
    let accent = app.accent;
    let motes = !cx.reduce_motion();
    let viewport = window.viewport_size();
    let mut body = div()
        .flex_1()
        .min_h(theme::unit(0.0))
        .w_full()
        .flex()
        .flex_col()
        .relative()
        .overflow_hidden();

    if app.store.backdrop {
        body = body.child(super::backdrop::show(super::backdrop::Backdrop {
            dominant: accent,
            width: viewport.width.as_f32(),
            height: viewport.height.as_f32(),
            motes,
            strength: f32::from(app.store.glow) / 100.0,
            mote: f32::from(app.store.mote) / 100.0,
        }));
    }

    if panel {
        body = body.child(player::panel(app, window, cx));
    }

    body = body.child(
        div()
            .flex_1()
            .min_h(theme::unit(0.0))
            .w_full()
            .flex()
            .flex_col()
            .py(theme::unit(if narrow { 0.0 } else { 24.0 }))
            // The phone has no inset here: every page carries its own, the way
            // Jota's shell leaves the horizontal padding to the page it hosts.
            .child(page(screen.cached(StyleRefinement::default().size_full()))),
    );
    root = root.child(body);

    // Overlays live in the area between the header and the phone navigation,
    // so a sheet's bottom edge lands on the nav instead of sliding under it.
    // This layer also reads pans first: the coast a dismissed sheet left
    // behind is dropped here, before the page under it can scroll with it.
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
            .on_scroll(|event, _, cx| sheet::leftover(event, cx))
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
