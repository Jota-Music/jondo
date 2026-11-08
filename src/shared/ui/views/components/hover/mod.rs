use std::cell::RefCell;
use std::collections::HashMap;
use std::time::{Duration, Instant};

use gpui::{
    App, ClickEvent, Div, ElementId, InteractiveElement, Stateful, StatefulInteractiveElement,
    Styled, Window,
};

const DURATION: Duration = Duration::from_millis(150);

#[derive(Clone, Copy)]
struct Fade {
    from: f32,
    target: f32,
    at: Instant,
}

impl Fade {
    fn value(&self, now: Instant) -> f32 {
        let t =
            (now.duration_since(self.at).as_secs_f32() / DURATION.as_secs_f32()).clamp(0.0, 1.0);
        self.from + (self.target - self.from) * t
    }
}

thread_local! {
    static FADES: RefCell<HashMap<ElementId, Fade>> = RefCell::new(HashMap::new());
}

pub fn value(id: &ElementId, now: Instant) -> f32 {
    FADES.with(|fades| {
        fades
            .borrow()
            .get(id)
            .map(|fade| fade.value(now))
            .unwrap_or(0.0)
    })
}

pub fn set(id: &ElementId, on: bool, now: Instant) {
    FADES.with(|fades| {
        let mut fades = fades.borrow_mut();
        let current = fades.get(id).map(|fade| fade.value(now)).unwrap_or(0.0);
        fades.insert(
            id.clone(),
            Fade {
                from: current,
                target: if on { 1.0 } else { 0.0 },
                at: now,
            },
        );
    });
}

/// A hover surface whose colour eases over 150ms instead of snapping.
///
/// A disabled surface keeps its layout and colour but drops the fade, the
/// pointer and the click, so it cannot look interactive when it is not.
pub fn hover(
    id: impl Into<ElementId>,
    enabled: bool,
    build: impl Fn(f32) -> Div + 'static,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let id = id.into();
    if !enabled {
        forget(&id);
        return build(0.0)
            .id(id.clone())
            .debug_selector(move || id.to_string())
            .cursor_default();
    }

    let now = Instant::now();
    let fade = value(&id, now);
    let named = id.clone();
    let mut element = build(fade)
        .id(id.clone())
        .debug_selector(move || named.to_string())
        .cursor_pointer();
    let hover_id = id.clone();
    element = element.on_hover(move |on: &bool, _, cx| {
        set(&hover_id, *on, Instant::now());
        cx.refresh_windows();
    });
    element.on_click(move |event, window, cx| on_click(event, window, cx))
}

fn forget(id: &ElementId) {
    FADES.with(|fades| fades.borrow_mut().remove(id));
}

/// Whether any hover fade still needs frames. A surface the pointer rests on is
/// settled, not running: only a fade that has not reached its target asks for a
/// frame, so a still cursor does not keep the window painting forever.
pub fn running() -> bool {
    let now = Instant::now();
    FADES.with(|fades| {
        let mut fades = fades.borrow_mut();
        let mut running = false;
        fades.retain(|_, fade| {
            let moving = (fade.value(now) - fade.target).abs() > 0.001;
            running |= moving;
            moving || fade.target > 0.0
        });
        running
    })
}

#[cfg(test)]
#[path = "hover.test.rs"]
mod tests;
