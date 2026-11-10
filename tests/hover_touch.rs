use gpui::{
    Context, InteractiveElement, IntoElement, Modifiers, ParentElement, Render, ScrollHandle,
    StatefulInteractiveElement, Styled, TestAppContext, TouchEvent, TouchId, TouchPhase, Window,
    div, point, px,
};
use gpui_pre as gpui;

use jondo::shared::ui::views::components::hover;
use jondo::shared::ui::views::design::theme;

struct Probe {
    scroll: ScrollHandle,
}

impl Render for Probe {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("scroll")
            .w(px(300.0))
            .h(px(300.0))
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .child(div().flex().flex_col().children((0..20).map(|index| {
                let id = gpui::ElementId::Name(format!("row-{index}").into());
                let probe = id.clone();
                div()
                    .id(id)
                    .h(px(50.0))
                    .w_full()
                    .bg(gpui::white())
                    .on_hover(move |on: &bool, window, cx| {
                        hover::set(
                            &probe,
                            *on,
                            std::time::Instant::now(),
                            window.last_input_was_touch(),
                        );
                        cx.refresh_windows();
                    })
            })))
    }
}

#[gpui::test]
fn a_touch_pan_lights_no_row_under_the_finger(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        theme::install(cx);
    });

    let scroll = ScrollHandle::new();
    let tracked = scroll.clone();
    let (_view, cx) = cx.add_window_view(move |_, _| Probe { scroll: tracked });
    cx.run_until_parked();

    cx.simulate_mouse_move(point(px(400.0), px(400.0)), None, Modifiers::default());
    cx.run_until_parked();
    // The probe's first layout can leave a row under the initial cursor, and a
    // fade still easing out reads as running: wait it out before the pan.
    std::thread::sleep(std::time::Duration::from_millis(200));
    cx.run_until_parked();
    assert!(!hover::running(), "no row is hovered before the pan");

    let start = point(px(150.0), px(120.0));
    cx.simulate_event(TouchEvent {
        id: TouchId(1),
        phase: TouchPhase::Started,
        position: start,
        ..Default::default()
    });
    cx.simulate_event(TouchEvent {
        id: TouchId(1),
        phase: TouchPhase::Moved,
        position: point(start.x, start.y - px(130.0)),
        ..Default::default()
    });
    cx.run_until_parked();
    cx.simulate_event(TouchEvent {
        id: TouchId(1),
        phase: TouchPhase::Ended,
        position: point(start.x, start.y - px(130.0)),
        ..Default::default()
    });
    cx.run_until_parked();

    assert!(scroll.offset().y < px(0.0), "the pan scrolls the list");
    assert!(
        !hover::running(),
        "a row scrolled under the finger must not stay lit"
    );
}
