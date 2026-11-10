use std::cell::RefCell;
use std::collections::HashMap;
use std::time::{Duration, Instant};

use gpui::{
    App, ClickEvent, Div, ElementId, InteractiveElement, MouseButton, Stateful,
    StatefulInteractiveElement, Styled, Window,
};

use super::paint::{self, Scope};

const DURATION: Duration = Duration::from_millis(150);

#[derive(Clone, Copy)]
struct Fade {
    from: f32,
    target: f32,
    at: Instant,
    /// Whether the surface lives in the route page, so the shell clock keeps
    /// repainting the page only while a page fade is in flight.
    page: bool,
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

pub fn set(id: &ElementId, on: bool, now: Instant, touch: bool) {
    set_in(Scope::Both, id, on, now, touch);
}

pub fn set_in(scope: Scope, id: &ElementId, on: bool, now: Instant, touch: bool) {
    if on && touch {
        return;
    }

    FADES.with(|fades| {
        let mut fades = fades.borrow_mut();
        let current = fades.get(id).map(|fade| fade.value(now)).unwrap_or(0.0);
        fades.insert(
            id.clone(),
            Fade {
                from: current,
                target: if on { 1.0 } else { 0.0 },
                at: now,
                // `Both` cannot say where the surface lives, so it keeps the
                // page painting: a stale page is worse than an extra frame.
                page: scope != Scope::Shell,
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
    hover_in(Scope::Both, id, enabled, build, on_click)
}

pub fn hover_in(
    scope: Scope,
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
            .cursor_default()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation());
    }

    let now = Instant::now();
    let fade = value(&id, now);
    let named = id.clone();
    // The surface owns the press: the row, card or overlay behind it must not
    // select, open or seek on the same click. Hover is left alone, so a
    // group-hover control that lives inside its own surface still shows.
    let mut element = build(fade)
        .id(id.clone())
        .debug_selector(move || named.to_string())
        .cursor_pointer()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation());
    let hover_id = id.clone();
    element = element.on_hover(move |on: &bool, window, cx| {
        set_in(
            scope,
            &hover_id,
            *on,
            Instant::now(),
            window.last_input_was_touch(),
        );
        paint::paint(scope, cx);
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

/// Whether a fade that lives in the route page still needs frames. A fade on
/// the shell must not keep repainting the cached page under it.
pub fn page_running() -> bool {
    let now = Instant::now();
    FADES.with(|fades| {
        fades
            .borrow()
            .values()
            .any(|fade| fade.page && (fade.value(now) - fade.target).abs() > 0.001)
    })
}

#[cfg(test)]
#[path = "hover.test.rs"]
mod tests;
