use std::cell::Cell;
use std::rc::Rc;

use gpui::{
    Context, IntoElement, Modifiers, MouseButton, ParentElement, Render,
    StatefulInteractiveElement, TestAppContext, Window, div, point, px,
};

use super::*;

#[test]
fn a_forgotten_surface_stops_fading() {
    let id = ElementId::Name("hover-test".into());
    let then = Instant::now() - DURATION;
    set(&id, true, then, false);
    assert!(value(&id, Instant::now()) > 0.9);
    forget(&id);
    assert_eq!(value(&id, Instant::now()), 0.0);
    assert!(!running());
}

struct Probe {
    under: Rc<Cell<usize>>,
    over: Rc<Cell<usize>>,
    lit: Rc<Cell<bool>>,
}

impl Render for Probe {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let under = self.under.clone();
        let over = self.over.clone();
        let lit = self.lit.clone();
        div()
            .relative()
            .size_full()
            .child(
                div()
                    .id("under")
                    .absolute()
                    .left_0()
                    .top_0()
                    .w(px(200.0))
                    .h(px(80.0))
                    .on_hover(move |on: &bool, _, _| lit.set(*on))
                    .on_click(move |_, _, _| under.set(under.get() + 1)),
            )
            .child(
                div()
                    .absolute()
                    .left_0()
                    .top_0()
                    .w(px(200.0))
                    .h(px(80.0))
                    .child(hover(
                        "over",
                        true,
                        |_| div().size_full(),
                        move |_, _, _| over.set(over.get() + 1),
                    )),
            )
    }
}

#[gpui::test]
fn a_surface_owns_its_rectangle(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        crate::shared::ui::views::design::theme::install(cx);
    });

    let under = Rc::new(Cell::new(0));
    let over = Rc::new(Cell::new(0));
    let lit = Rc::new(Cell::new(false));
    let (_view, cx) = cx.add_window_view({
        let under = under.clone();
        let over = over.clone();
        let lit = lit.clone();
        move |_, _| Probe { under, over, lit }
    });
    cx.run_until_parked();

    let center = point(px(100.0), px(40.0));
    cx.simulate_mouse_move(center, None, Modifiers::default());
    cx.run_until_parked();
    assert!(
        lit.get(),
        "the surface on top must not steal the hover behind"
    );
    cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
    cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
    cx.run_until_parked();

    assert_eq!(over.get(), 1, "the surface on top receives the click");
    assert_eq!(under.get(), 0, "the element behind must not receive it too");
}
