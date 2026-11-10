use std::cell::Cell;
use std::rc::Rc;

use gpui::prelude::FluentBuilder as _;
use gpui::{
    Context, InteractiveElement, IntoElement, Modifiers, MouseButton, ParentElement, Render,
    Styled, TestAppContext, TouchEvent, TouchId, TouchPhase, VisualTestContext, Window, div, px,
};

use super::{Bar, TRACK};
use crate::shared::ui::views::design::theme;

struct Probe {
    changes: Rc<Cell<usize>>,
    covered: bool,
}

impl Render for Probe {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let changes = self.changes.clone();
        let bar = div()
            .debug_selector(|| "bar".to_owned())
            .w(px(200.0))
            .h(px(8.0))
            .child(Bar::new(
                0.5,
                Some(200.0),
                8.0,
                theme::TEXT,
                TRACK,
                4.0,
                true,
                false,
                move |_, _, _, _| changes.set(changes.get() + 1),
            ));

        div()
            .relative()
            .size_full()
            .child(div().absolute().left_0().top_0().child(bar))
            .when(self.covered, |root| {
                root.child(
                    div()
                        .debug_selector(|| "cover".to_owned())
                        .absolute()
                        .left_0()
                        .top_0()
                        .w(px(200.0))
                        .h(px(80.0))
                        .occlude(),
                )
            })
    }
}

fn probe(cx: &mut TestAppContext, covered: bool) -> (Rc<Cell<usize>>, &mut VisualTestContext) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        theme::install(cx);
    });

    let changes = Rc::new(Cell::new(0));
    let probe = changes.clone();
    let (_view, cx) = cx.add_window_view(move |_, _| Probe {
        changes: probe,
        covered,
    });
    cx.run_until_parked();
    (changes, cx)
}

fn tap(cx: &mut gpui::VisualTestContext, at: gpui::Point<gpui::Pixels>) {
    for phase in [TouchPhase::Started, TouchPhase::Ended] {
        cx.simulate_event(TouchEvent {
            id: TouchId(1),
            phase,
            position: at,
            ..Default::default()
        });
    }
    cx.run_until_parked();
}

#[gpui::test]
fn a_press_reaches_the_bar_when_it_is_on_top(cx: &mut TestAppContext) {
    let (changes, cx) = probe(cx, false);

    let center = cx.debug_bounds("bar").expect("the bar paints").center();
    cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
    cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
    cx.run_until_parked();

    assert!(changes.get() > 0, "a press on the bar arms its seek");
}

#[gpui::test]
fn a_press_does_not_reach_the_bar_through_a_cover(cx: &mut TestAppContext) {
    let (changes, cx) = probe(cx, true);

    let center = cx.debug_bounds("bar").expect("the bar paints").center();
    cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
    cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
    cx.run_until_parked();
    assert_eq!(changes.get(), 0, "a cover owns the press");

    tap(cx, center);
    assert_eq!(changes.get(), 0, "a tap under a cover must not seek");
}

#[gpui::test]
fn a_finger_below_the_line_still_grabs_the_bar(cx: &mut TestAppContext) {
    let (changes, cx) = probe(cx, false);

    let center = cx.debug_bounds("bar").expect("the bar paints").center();
    tap(cx, gpui::point(center.x, center.y + px(16.0)));
    assert!(
        changes.get() > 0,
        "a touch aimed at the thumb reaches the band around the line"
    );
}
