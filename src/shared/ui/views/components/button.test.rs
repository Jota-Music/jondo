use std::cell::Cell;
use std::rc::Rc;

use gpui::{
    Context, IntoElement, KeyDownEvent, KeyUpEvent, Keystroke, Modifiers, ParentElement, Render,
    Styled, TestAppContext, Window, div,
};

use super::{icon_button, icons, theme};

struct Probe {
    hit: Rc<Cell<bool>>,
}

impl Render for Probe {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let hit = self.hit.clone();
        div().size_full().child(icon_button(
            "probe",
            icons::PLAY,
            16.0,
            theme::TEXT,
            32.0,
            true,
            move |_, _, _| hit.set(true),
        ))
    }
}

#[gpui::test]
fn a_focused_button_activates_with_enter(cx: &mut TestAppContext) {
    let hit = Rc::new(Cell::new(false));
    let pressed = hit.clone();

    cx.update(|cx| {
        gpui_kit::init(cx);
        theme::install(cx);
    });

    let (_view, cx) = cx.add_window_view(|_, _| Probe { hit: pressed });
    cx.run_until_parked();

    let focused = cx.update(|window, cx| {
        window.focus_next(cx);
        window.focused(cx).is_some()
    });
    assert!(focused, "the button must be a tab stop");

    let stroke = Keystroke {
        key: "enter".to_owned(),
        key_char: None,
        modifiers: Modifiers::default(),
    };
    cx.simulate_event(KeyDownEvent {
        keystroke: stroke.clone(),
        is_held: false,
        prefer_character_input: false,
    });
    cx.simulate_event(KeyUpEvent { keystroke: stroke });

    assert!(hit.get(), "Enter on a focused button must activate it");
}
