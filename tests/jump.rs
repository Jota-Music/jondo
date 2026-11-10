//! The jump pill floats over the queue and the song lists. Its hitbox must own
//! the pointer, or the row it covers keeps lighting its hover while the cursor
//! rests on the pill.

use std::path::PathBuf;
use std::time::Instant;

use gpui::{ElementId, Modifiers, TestAppContext, VisualTestContext};
use gpui_pre as gpui;
use jondo::shared::music::domain::Song;
use jondo::shared::storage::Store;
use jondo::shared::ui::views::components::hover;
use jondo::shell::app::Jondo;

fn home() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("jondo-jump-{}", std::process::id()));
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

#[gpui::test]
fn the_pill_owns_the_pointer_instead_of_the_row_beneath_it(cx: &mut TestAppContext) {
    home();
    let songs: Vec<Song> = (0..60)
        .map(|index| Song {
            id: format!("local:song-{index}"),
            name: format!("Song {index}"),
            duration: 120,
            ..Default::default()
        })
        .collect();

    let mut store = Store::load();
    store.queue = songs.clone();
    store.queue_index = 0;

    cx.update(|cx| {
        gpui_kit::init(cx);
        jondo::shared::ui::views::design::theme::install(cx);
    });

    let (app, cx) = cx.add_window_view(|window, cx| Jondo::boot(store, window, cx));
    cx.run_until_parked();
    frame(cx);

    app.update(cx, |app, cx| app.open_queue(cx));
    frame(cx);
    app.update(cx, |app, cx| {
        app.scroll("queue")
            .scroll_to_item(59, gpui::ScrollStrategy::Top);
        cx.notify();
    });
    frame(cx);
    frame(cx);

    let pill = cx
        .debug_bounds("queue-jump")
        .expect("the playing row is out of view, so the pill must paint");
    cx.simulate_mouse_move(pill.center(), None, Modifiers::default());
    std::thread::sleep(std::time::Duration::from_millis(250));
    frame(cx);

    let lit: Vec<usize> = (0..songs.len())
        .filter(|index| {
            hover::value(
                &ElementId::Name(format!("queue-row-{index}").into()),
                Instant::now(),
            ) > 0.0
        })
        .collect();
    assert!(
        lit.is_empty(),
        "the cursor rests on the pill, so no row under it may keep its hover: {lit:?}"
    );
}
