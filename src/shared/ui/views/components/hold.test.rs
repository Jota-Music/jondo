use std::cell::Cell;
use std::rc::Rc;

use gpui::{
    Context, InteractiveElement, IntoElement, LongPressEvent, ParentElement, Render, Styled,
    TestAppContext, TouchPhase, Window, div, point, px,
};

use super::hold;

struct Probe {
    hits: Rc<Cell<usize>>,
}

impl Render for Probe {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let hits = self.hits.clone();
        div().size_full().child(
            div()
                .debug_selector(|| "hold-target".to_owned())
                .absolute()
                .left_0()
                .top_0()
                .w(px(200.0))
                .h(px(80.0))
                .child(hold(move |_, _, _| hits.set(hits.get() + 1))),
        )
    }
}

fn press(cx: &mut gpui::VisualTestContext, at: gpui::Point<gpui::Pixels>) {
    for phase in [TouchPhase::Started, TouchPhase::Ended] {
        cx.simulate_event(LongPressEvent {
            phase,
            start_position: at,
            position: at,
        });
    }
}

#[gpui::test]
fn a_long_press_over_the_trigger_runs_its_action(cx: &mut TestAppContext) {
    let hits = Rc::new(Cell::new(0));
    let (_, cx) = cx.add_window_view({
        let hits = hits.clone();
        move |_, _| Probe { hits }
    });
    cx.run_until_parked();

    let center = cx
        .debug_bounds("hold-target")
        .expect("the trigger paints")
        .center();
    press(cx, center);

    assert_eq!(hits.get(), 1);
}

#[gpui::test]
fn a_long_press_outside_the_trigger_is_ignored(cx: &mut TestAppContext) {
    let hits = Rc::new(Cell::new(0));
    let (_, cx) = cx.add_window_view({
        let hits = hits.clone();
        move |_, _| Probe { hits }
    });
    cx.run_until_parked();

    press(cx, point(px(600.0), px(400.0)));

    assert_eq!(hits.get(), 0);
}
