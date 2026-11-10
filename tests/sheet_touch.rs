use std::path::PathBuf;

use gpui::{TestAppContext, VisualTestContext};
use gpui_pre as gpui;
use jondo::shell::app::Jondo;

fn home() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("jondo-sheet-touch-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    jondo::shared::storage::use_home(&dir);
    dir
}

fn frame(cx: &mut VisualTestContext) {
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
}

fn touch(phase: gpui::TouchPhase, x: gpui::Pixels, y: gpui::Pixels) -> gpui::TouchEvent {
    gpui::TouchEvent {
        id: gpui::TouchId(1),
        phase,
        position: gpui::point(x, y),
        predicted_position: None,
        force: None,
    }
}

#[gpui::test]
fn a_touch_drag_on_the_grip_dismisses_the_queue(cx: &mut TestAppContext) {
    home();
    let store = jondo::shared::storage::Store::load();

    cx.update(|cx| {
        gpui_kit::init(cx);
        jondo::shared::ui::views::design::theme::install(cx);
    });

    let (app, cx) = cx.add_window_view(|window, cx| Jondo::boot(store, window, cx));
    cx.run_until_parked();
    cx.simulate_resize(gpui::size(gpui::px(390.0), gpui::px(844.0)));
    frame(cx);

    app.update(cx, |app, cx| app.open_queue(cx));
    // The first frame measures the panel, the next starts the entrance, which
    // runs on the real clock.
    frame(cx);
    frame(cx);
    std::thread::sleep(std::time::Duration::from_millis(350));
    frame(cx);
    frame(cx);

    let content = cx
        .debug_bounds("queue-sheet")
        .expect("the queue sheet must paint its content");
    let x = content.center().x;
    // The grab strip (`CLOSE_TOP + CLOSE_SIZE` tall) sits just above the body.
    let start = content.top() - gpui::px(26.0);

    cx.simulate_event(touch(gpui::TouchPhase::Started, x, start));
    for step in 1..=8 {
        cx.simulate_event(touch(
            gpui::TouchPhase::Moved,
            x,
            start + gpui::px(20.0 * step as f32),
        ));
    }
    // Let the frames settle onto the finger before measuring.
    for _ in 0..8 {
        std::thread::sleep(std::time::Duration::from_millis(20));
        frame(cx);
    }
    let pulled = cx
        .debug_bounds("queue-sheet")
        .expect("the sheet must follow the finger");
    let travel = pulled.top() - content.top();
    assert!(
        (travel - gpui::px(160.0)).abs() < gpui::px(2.0),
        "the sheet must travel exactly with the finger: {content:?} -> {pulled:?}"
    );

    cx.simulate_event(touch(gpui::TouchPhase::Ended, x, start + gpui::px(160.0)));
    // The exit runs on the sheet's own frames, so give a release the time it
    // takes to visibly move before asking where the sheet is.
    std::thread::sleep(std::time::Duration::from_millis(60));
    frame(cx);

    let dismissed = cx
        .debug_bounds("queue-sheet")
        .is_none_or(|after| after.top() > pulled.top() + gpui::px(20.0));
    assert!(
        dismissed,
        "releasing past the close line must dismiss the sheet"
    );
}

#[gpui::test]
fn an_upward_touch_pan_does_not_pull_the_sheet(cx: &mut TestAppContext) {
    home();
    let store = jondo::shared::storage::Store::load();

    cx.update(|cx| {
        gpui_kit::init(cx);
        jondo::shared::ui::views::design::theme::install(cx);
    });

    let (app, cx) = cx.add_window_view(|window, cx| Jondo::boot(store, window, cx));
    cx.run_until_parked();
    cx.simulate_resize(gpui::size(gpui::px(390.0), gpui::px(844.0)));
    frame(cx);

    app.update(cx, |app, cx| app.open_queue(cx));
    frame(cx);
    frame(cx);
    std::thread::sleep(std::time::Duration::from_millis(350));
    frame(cx);
    frame(cx);

    let content = cx
        .debug_bounds("queue-sheet")
        .expect("the queue sheet must paint its content");
    let x = content.center().x;
    let start = content.top() + gpui::px(24.0);

    cx.simulate_event(touch(gpui::TouchPhase::Started, x, start));
    for step in 1..=8 {
        cx.simulate_event(touch(
            gpui::TouchPhase::Moved,
            x,
            start - gpui::px(20.0 * step as f32),
        ));
    }
    frame(cx);

    let after = cx
        .debug_bounds("queue-sheet")
        .expect("an upward pan must leave the sheet in place");
    assert!(
        (after.top() - content.top()).abs() < gpui::px(1.0),
        "an upward pan scrolls the body instead of pulling the sheet: {content:?} -> {after:?}"
    );
}

#[gpui::test]
fn a_scrolled_body_keeps_its_downward_pan(cx: &mut TestAppContext) {
    home();
    let store = jondo::shared::storage::Store::load();

    cx.update(|cx| {
        gpui_kit::init(cx);
        jondo::shared::ui::views::design::theme::install(cx);
    });

    let (app, cx) = cx.add_window_view(|window, cx| Jondo::boot(store, window, cx));
    cx.run_until_parked();
    cx.simulate_resize(gpui::size(gpui::px(390.0), gpui::px(844.0)));
    frame(cx);

    app.update(cx, |app, cx| app.open_queue(cx));
    frame(cx);
    frame(cx);
    std::thread::sleep(std::time::Duration::from_millis(350));
    frame(cx);
    frame(cx);

    let content = cx
        .debug_bounds("queue-sheet")
        .expect("the queue sheet must paint its content");
    // An empty queue has no scroll range, so force one to prove the body keeps
    // its pan once it is away from the top.
    app.update(cx, |app, _| {
        let scroll = app.scroll("queue");
        scroll
            .0
            .borrow()
            .base_handle
            .set_offset(gpui::point(gpui::px(0.0), gpui::px(-120.0)));
    });

    let x = content.center().x;
    let start = content.top() + gpui::px(24.0);

    cx.simulate_event(touch(gpui::TouchPhase::Started, x, start));
    let offset = app.update(cx, |app, _| {
        app.scroll("queue").0.borrow().base_handle.offset().y
    });
    assert!(
        offset < gpui::px(0.0),
        "the body must still be scrolled when the pan starts: {offset:?}"
    );

    for step in 1..=8 {
        cx.simulate_event(touch(
            gpui::TouchPhase::Moved,
            x,
            start + gpui::px(20.0 * step as f32),
        ));
    }
    frame(cx);

    let after = cx
        .debug_bounds("queue-sheet")
        .expect("a scrolled body must not drag the sheet");
    assert!(
        (after.top() - content.top()).abs() < gpui::px(1.0),
        "a scrolled body keeps its downward pan: {content:?} -> {after:?}"
    );
}

#[gpui::test]
fn a_touch_drag_on_the_sheet_body_dismisses_the_queue(cx: &mut TestAppContext) {
    home();
    let store = jondo::shared::storage::Store::load();

    cx.update(|cx| {
        gpui_kit::init(cx);
        jondo::shared::ui::views::design::theme::install(cx);
    });

    let (app, cx) = cx.add_window_view(|window, cx| Jondo::boot(store, window, cx));
    cx.run_until_parked();
    cx.simulate_resize(gpui::size(gpui::px(390.0), gpui::px(844.0)));
    frame(cx);

    app.update(cx, |app, cx| app.open_queue(cx));
    frame(cx);
    frame(cx);
    std::thread::sleep(std::time::Duration::from_millis(350));
    frame(cx);
    frame(cx);

    let content = cx
        .debug_bounds("queue-sheet")
        .expect("the queue sheet must paint its content");
    let x = content.center().x;
    // The header row sits inside the body, below the grab strip.
    let start = content.top() + gpui::px(24.0);

    cx.simulate_event(touch(gpui::TouchPhase::Started, x, start));
    for step in 1..=8 {
        cx.simulate_event(touch(
            gpui::TouchPhase::Moved,
            x,
            start + gpui::px(20.0 * step as f32),
        ));
    }
    // Let the frames settle onto the finger before measuring.
    for _ in 0..8 {
        std::thread::sleep(std::time::Duration::from_millis(20));
        frame(cx);
    }
    let pulled = cx
        .debug_bounds("queue-sheet")
        .expect("the sheet must follow the finger");
    let travel = pulled.top() - content.top();
    assert!(
        (travel - gpui::px(160.0)).abs() < gpui::px(2.0),
        "a downward drag on the sheet body must travel with the finger: {content:?} -> {pulled:?}"
    );
}
