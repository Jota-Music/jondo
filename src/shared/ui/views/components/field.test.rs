use gpui::{
    AppContext as _, Context, Entity, InteractiveElement, IntoElement, ParentElement, Render,
    ScrollHandle, StatefulInteractiveElement, Styled, TestAppContext, TouchEvent, TouchId,
    TouchPhase, Window, div, point, px,
};
use gpui_kit::component::input::InputState;

use super::text;
use crate::shared::ui::views::design::theme;

struct Probe {
    scroll: ScrollHandle,
    field: Entity<InputState>,
}

impl Render for Probe {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("scroll")
            .size_full()
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .child(
                div().h(px(2000.0)).child(
                    div()
                        .id("probe-field")
                        .debug_selector(|| "probe-field".to_owned())
                        .w(px(300.0))
                        .child(text(&self.field)),
                ),
            )
    }
}

#[gpui::test]
fn a_touch_pan_started_on_a_field_scrolls_the_page(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        theme::install(cx);
    });

    let scroll = ScrollHandle::new();
    let tracked = scroll.clone();
    let (_view, cx) = cx.add_window_view(move |window, cx| Probe {
        scroll: tracked,
        field: cx.new(|cx| InputState::new(window, cx)),
    });
    cx.run_until_parked();

    let start = cx
        .debug_bounds("probe-field")
        .expect("the field must paint")
        .center();

    cx.simulate_event(TouchEvent {
        id: TouchId(1),
        phase: TouchPhase::Started,
        position: start,
        ..Default::default()
    });
    cx.simulate_event(TouchEvent {
        id: TouchId(1),
        phase: TouchPhase::Moved,
        position: point(start.x, start.y - px(120.0)),
        ..Default::default()
    });
    cx.run_until_parked();

    assert!(
        scroll.offset().y < px(0.0),
        "a finger dragged from a field must scroll the page, not be swallowed by the field"
    );
}
