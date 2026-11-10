use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gpui::{
    Context, InteractiveElement, IntoElement, ParentElement, Pixels, Point, Render, Styled,
    TestAppContext, TouchEvent, TouchId, TouchPhase, VisualTestContext, Window, div, point, px,
    size,
};

use super::Ring;
use crate::shared::ui::views::design::theme;

type Seeks = Rc<RefCell<Vec<(f32, bool)>>>;

struct Probe {
    seeks: Seeks,
    dragging: Rc<Cell<bool>>,
}

impl Render for Probe {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let seeks = self.seeks.clone();
        let dragging = self.dragging.clone();
        div()
            .debug_selector(|| "disc".to_owned())
            .size(px(240.0))
            .child(Ring::new(
                None,
                0.25,
                theme::TEXT,
                true,
                dragging.get(),
                move |value, commit, _, _| {
                    seeks.borrow_mut().push((value, commit));
                    dragging.set(!commit);
                },
            ))
    }
}

fn touch(phase: TouchPhase, at: Point<Pixels>) -> TouchEvent {
    TouchEvent {
        id: TouchId(1),
        phase,
        position: at,
        ..Default::default()
    }
}

fn probe(cx: &mut TestAppContext) -> (Seeks, &mut VisualTestContext) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        theme::install(cx);
    });

    let seeks = Seeks::default();
    let recorded = seeks.clone();
    let (_view, cx) = cx.add_window_view(move |_, _| Probe {
        seeks: recorded,
        dragging: Rc::new(Cell::new(false)),
    });
    cx.run_until_parked();
    cx.simulate_resize(size(px(400.0), px(400.0)));
    cx.run_until_parked();
    (seeks, cx)
}

fn frame(cx: &mut VisualTestContext) {
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
}

#[gpui::test]
fn a_touch_drag_on_the_ring_previews_and_commits(cx: &mut TestAppContext) {
    let (seeks, cx) = probe(cx);

    let bounds = cx.debug_bounds("disc").expect("the disc paints");
    let center = bounds.center();
    let radius = bounds.size.width.as_f32() * 0.5 - 4.0;
    let right = point(center.x + px(radius), center.y);
    let top = point(center.x, center.y - px(radius));
    let left = point(center.x - px(radius), center.y);

    cx.simulate_event(touch(TouchPhase::Started, right));
    frame(cx);
    cx.simulate_event(touch(TouchPhase::Moved, top));
    cx.simulate_event(touch(TouchPhase::Ended, left));
    cx.run_until_parked();

    let seeks = seeks.borrow();
    assert_eq!(
        seeks.first().copied(),
        Some((0.25, false)),
        "a press on the ring previews its angle: {seeks:?}"
    );
    assert_eq!(
        seeks.last().copied(),
        Some((0.75, true)),
        "the release commits the angle under the finger: {seeks:?}"
    );
}

#[gpui::test]
fn a_touch_drag_off_the_ring_does_not_seek(cx: &mut TestAppContext) {
    let (seeks, cx) = probe(cx);

    let center = cx.debug_bounds("disc").expect("the disc paints").center();

    cx.simulate_event(touch(TouchPhase::Started, center));
    cx.simulate_event(touch(TouchPhase::Moved, center));
    cx.simulate_event(touch(TouchPhase::Ended, center));
    cx.run_until_parked();

    assert!(
        seeks.borrow().is_empty(),
        "the center of the disc belongs to the cover, not the ring"
    );
}
