//! What the shell repaints, and what it leaves alone: a motion frame, a
//! shell-only change or a hover on the chrome keeps the cached page. A page
//! change or a fade that lives in the page repaints it.

use std::path::PathBuf;

use gpui::{ElementId, TestAppContext, VisualTestContext};

use crate::shared::storage::{LocalPlaylist, Store};
use crate::shared::ui::views::components::hover;
use crate::shared::ui::views::components::paint::{self, Scope};
use crate::shared::ui::views::design::theme;
use crate::shell::app::{Jondo, motion};

fn home() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("jondo-render-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    crate::shared::storage::use_home(&dir);
    dir
}

fn frame(cx: &mut VisualTestContext) {
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
}

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
    });
}

fn shelf() -> Store {
    let mut store = Store::load();
    for index in 0..3 {
        store.playlists.push(LocalPlaylist {
            id: format!("local:mix-{index}"),
            name: format!("Mix {index}"),
            songs: Vec::new(),
            modified: 0,
        });
    }

    store
}

#[gpui::test]
fn a_page_hover_paints_a_motion_frame(cx: &mut TestAppContext) {
    home();
    cx.update(|cx| {
        gpui_kit::init(cx);
        theme::install(cx);
    });

    let (app, cx) = cx.add_window_view(|window, cx| Jondo::boot(shelf(), window, cx));
    cx.run_until_parked();
    frame(cx);
    frame(cx);

    // A notify raised while the page is being drawn lands in the next frame, so
    // the first motion frame settles the pending paint and the second one must
    // keep the cached page.
    motion::note();
    frame(cx);
    let settled = app.update(cx, |app, _| app.covers.page_frames());
    motion::note();
    frame(cx);
    assert_eq!(
        app.update(cx, |app, _| app.covers.page_frames()),
        settled,
        "a motion frame with nothing moving in the page must keep the cached page"
    );

    // A fade in the page must repaint it on the very same kind of frame.
    hover::set(
        &ElementId::Name("filter-youtube".into()),
        true,
        std::time::Instant::now(),
        false,
    );
    assert!(hover::running(), "the fade must be in flight");
    // A notify raised during the page's own draw lands in the next frame, so
    // the second motion frame is the one that proves the first one asked for a
    // page paint.
    motion::note();
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
    });
    motion::note();
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
    });
    assert_eq!(
        app.update(cx, |app, _| app.covers.page_frames()),
        settled + 1,
        "a hover fade lives in the page, so a motion frame must repaint it"
    );
}

#[gpui::test]
fn a_shell_notify_keeps_the_cached_page(cx: &mut TestAppContext) {
    home();
    cx.update(|cx| {
        gpui_kit::init(cx);
        theme::install(cx);
    });

    let (app, cx) = cx.add_window_view(|window, cx| Jondo::boot(shelf(), window, cx));
    cx.run_until_parked();
    frame(cx);
    frame(cx);

    // A notify raised while the page is being drawn lands in the next frame,
    // so settle the pending paint before taking the baseline.
    motion::note();
    frame(cx);
    let before = app.update(cx, |app, _| app.covers.page_frames());
    app.update(cx, |app, cx| app.notify_shell(cx));
    draw(cx);
    let after = app.update(cx, |app, _| app.covers.page_frames());
    assert_eq!(
        after, before,
        "a shell-only change must leave the route page cached"
    );
}

#[gpui::test]
fn a_shell_hover_keeps_the_cached_page(cx: &mut TestAppContext) {
    home();
    cx.update(|cx| {
        gpui_kit::init(cx);
        theme::install(cx);
    });

    let (app, cx) = cx.add_window_view(|window, cx| Jondo::boot(shelf(), window, cx));
    cx.run_until_parked();
    frame(cx);
    frame(cx);

    motion::note();
    frame(cx);
    let settled = app.update(cx, |app, _| app.covers.page_frames());
    let id = ElementId::Name("shell-hover".into());
    hover::set_in(Scope::Shell, &id, true, std::time::Instant::now(), false);
    cx.update(|_, cx| paint::paint(Scope::Shell, cx));

    motion::note();
    draw(cx);
    motion::note();
    draw(cx);
    assert_eq!(
        app.update(cx, |app, _| app.covers.page_frames()),
        settled,
        "a fade on the chrome must not repaint the page under it"
    );
}

#[gpui::test]
fn a_paint_from_inside_an_app_update_is_deferred(cx: &mut TestAppContext) {
    home();
    cx.update(|cx| {
        gpui_kit::init(cx);
        theme::install(cx);
    });

    let (app, cx) = cx.add_window_view(|window, cx| Jondo::boot(shelf(), window, cx));
    cx.run_until_parked();
    frame(cx);

    // A hover declared with `cx.listener` runs while the app is already being
    // updated. The painter must defer instead of updating the same entity.
    app.update(cx, |_, cx| paint::paint(Scope::Page, cx));
    cx.run_until_parked();
}

#[gpui::test]
fn a_page_paint_repaints_only_the_page(cx: &mut TestAppContext) {
    home();
    cx.update(|cx| {
        gpui_kit::init(cx);
        theme::install(cx);
    });

    let (app, cx) = cx.add_window_view(|window, cx| Jondo::boot(shelf(), window, cx));
    cx.run_until_parked();
    frame(cx);
    frame(cx);

    motion::note();
    frame(cx);
    let before = app.update(cx, |app, _| app.covers.page_frames());
    app.update(cx, |app, cx| app.paint_page(cx));
    draw(cx);
    let after = app.update(cx, |app, _| app.covers.page_frames());
    assert!(after > before, "the page must repaint");
}

/// A phone-sized window, which is the layout the sheet's own touch paths live
/// in: above 640px the sheet is a desktop dialog and takes none of them.
fn phone(cx: &mut VisualTestContext) {
    let handle = cx.update(|window, _| window.window_handle());
    cx.simulate_window_resize(handle, gpui::size(gpui::px(390.0), gpui::px(844.0)));
    frame(cx);
}

fn touch(cx: &mut VisualTestContext, phase: gpui::TouchPhase, at: gpui::Point<gpui::Pixels>) {
    cx.simulate_event(gpui::TouchEvent {
        id: gpui::TouchId(1),
        phase,
        position: at,
        ..Default::default()
    });
}

fn pan(cx: &mut VisualTestContext, from: gpui::Point<gpui::Pixels>, to: gpui::Point<gpui::Pixels>) {
    touch(cx, gpui::TouchPhase::Started, from);
    touch(cx, gpui::TouchPhase::Moved, to);
}

#[gpui::test]
fn a_pan_inside_a_sheet_is_a_motion_frame(cx: &mut TestAppContext) {
    home();
    cx.update(|cx| {
        gpui_kit::init(cx);
        theme::install(cx);
    });

    let (app, cx) = cx.add_window_view(|window, cx| Jondo::boot(shelf(), window, cx));
    cx.run_until_parked();
    phone(cx);

    app.update(cx, |app, cx| {
        app.ui.create_open = true;
        cx.notify();
    });
    frame(cx);
    frame(cx);

    let from = gpui::point(gpui::px(195.0), gpui::px(600.0));
    let to = gpui::point(gpui::px(195.0), gpui::px(540.0));

    // The first touch of a session flips the window's input modality, and that
    // refresh ignores every cache. Spend it before the pan is judged.
    touch(cx, gpui::TouchPhase::Started, from);
    touch(cx, gpui::TouchPhase::Cancelled, from);
    frame(cx);
    motion::took();

    // A finger panning inside the modal. The scrim covers the route, so the
    // frame only moves the sheet or scrolls its body: saying so is what keeps
    // the page behind the scrim out of it.
    pan(cx, from, to);

    assert!(
        motion::took(),
        "a pan inside a sheet must mark the frame as motion, or the route behind the scrim is rebuilt on every sample"
    );
}

#[gpui::test]
fn a_sheet_takes_itself_down_once_the_exit_is_done(cx: &mut TestAppContext) {
    home();
    cx.update(|cx| {
        gpui_kit::init(cx);
        theme::install(cx);
    });

    let (app, cx) = cx.add_window_view(|window, cx| Jondo::boot(shelf(), window, cx));
    cx.run_until_parked();
    phone(cx);

    app.update(cx, |app, cx| {
        app.ui.create_open = true;
        cx.notify();
    });
    frame(cx);
    assert!(
        cx.debug_bounds("create-submit").is_some(),
        "the sheet paints its body"
    );

    app.update(cx, |app, cx| {
        app.ui.closing = Some(std::time::Instant::now());
        cx.notify();
    });
    frame(cx);
    assert!(
        cx.debug_bounds("create-submit").is_some(),
        "a closing sheet stays mounted for its exit"
    );

    // The clock no longer wakes for a closing sheet: the sheet asks for its own
    // frames, so nothing but the frame after the exit takes it down.
    std::thread::sleep(crate::shell::views::sheet::DURATION + std::time::Duration::from_millis(40));
    frame(cx);
    frame(cx);
    assert!(
        cx.debug_bounds("create-submit").is_none(),
        "the sheet leaves once the caller's mount window is over"
    );
}

#[gpui::test]
fn a_dialog_drags_by_its_own_surface(cx: &mut TestAppContext) {
    home();
    cx.update(|cx| {
        gpui_kit::init(cx);
        theme::install(cx);
    });

    // The default window is wide, so the sheet is the centred dialog and its
    // mouse drag is the path under test.
    let (app, cx) = cx.add_window_view(|window, cx| Jondo::boot(shelf(), window, cx));
    cx.run_until_parked();
    frame(cx);

    app.update(cx, |app, cx| {
        app.ui.create_open = true;
        cx.notify();
    });
    frame(cx);
    frame(cx);

    let button = cx
        .debug_bounds("create-submit")
        .expect("the dialog paints its body");
    let start = button.center() - gpui::point(gpui::px(0.0), gpui::px(40.0));

    cx.simulate_mouse_down(start, gpui::MouseButton::Left, gpui::Modifiers::default());
    for step in 1..=6 {
        let at = start + gpui::point(gpui::px(0.0), gpui::px(20.0 * step as f32));
        cx.simulate_mouse_move(at, gpui::MouseButton::Left, gpui::Modifiers::default());
    }
    frame(cx);

    let pulled = cx
        .debug_bounds("create-submit")
        .expect("a dragged dialog keeps painting its body");
    let travel = pulled.top() - button.top();
    assert!(
        travel > gpui::px(80.0) && travel < gpui::px(140.0),
        "the dialog must follow the pointer: {button:?} -> {pulled:?}"
    );

    cx.simulate_mouse_up(
        start + gpui::point(gpui::px(0.0), gpui::px(120.0)),
        gpui::MouseButton::Left,
        gpui::Modifiers::default(),
    );
    assert!(
        app.update(cx, |app, _| app.ui.closing.is_some()),
        "releasing past the close line must dismiss the dialog"
    );
}
