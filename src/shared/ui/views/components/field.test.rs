use gpui::{
    AppContext as _, Context, Entity, InteractiveElement, IntoElement, ParentElement, Render,
    ScrollHandle, StatefulInteractiveElement, Styled, TestAppContext, TouchEvent, TouchId,
    TouchPhase, Window, div, point, px,
};
use gpui_kit::component::input::InputState;

use super::{secret, text};
use crate::shared::ui::views::design::theme;

struct Probe {
    scroll: ScrollHandle,
    field: Entity<InputState>,
    masked: bool,
}

impl Render for Probe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
                        .child(if self.masked {
                            secret(&self.field, cx).into_any_element()
                        } else {
                            text(&self.field).into_any_element()
                        }),
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
        masked: false,
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

#[gpui::test]
fn a_thumb_sized_eye_flips_the_mask_and_keeps_the_caret(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        theme::install(cx);
    });

    let (probe, cx) = cx.add_window_view(|window, cx| {
        let field = cx.new(|cx| InputState::new(window, cx));
        field.update(cx, |state, cx| state.set_masked(true, window, cx));
        Probe {
            scroll: ScrollHandle::new(),
            field,
            masked: true,
        }
    });
    cx.run_until_parked();

    let eye = cx
        .debug_bounds("secret-reveal")
        .expect("the eye must paint on a secret field");
    assert!(
        eye.size.width >= theme::unit(32.0) - theme::unit(0.5),
        "the eye is a thumb target, not a 20px glyph: {:?}",
        eye.size
    );

    let field = probe.read_with(cx, |probe, _| probe.field.clone());
    let center = eye.center();
    cx.simulate_mouse_down(center, gpui::MouseButton::Left, gpui::Modifiers::default());
    cx.simulate_mouse_up(center, gpui::MouseButton::Left, gpui::Modifiers::default());
    cx.run_until_parked();

    assert!(
        !field.read_with(cx, |state, _| state.presentation().is_masked()),
        "the eye must reveal the value"
    );
    assert!(
        cx.update(|window, cx| field
            .read(cx)
            .presentation()
            .focus_handle()
            .is_focused(window)),
        "revealing the value must leave the caret in the field, not drop the keyboard"
    );
}
