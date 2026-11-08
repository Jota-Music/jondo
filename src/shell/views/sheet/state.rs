use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Instant;

use gpui::{Context, ElementId, IntoElement, Render, Window, div};

use crate::shell::app::Jondo;

pub(super) type Close = Rc<dyn Fn(&mut Jondo, &mut Window, &mut Context<Jondo>)>;

pub(super) struct Motion {
    pub(super) from: f32,
    pub(super) at: Instant,
}

#[derive(Default)]
pub(super) struct State {
    pub(super) height: f32,
    pub(super) down_y: f32,
    pub(super) active: bool,
    pub(super) offset: f32,
    pub(super) velocity: f32,
    pub(super) moved_at: Option<Instant>,
    pub(super) settle: Option<Motion>,
    pub(super) dismissed: Option<f32>,
    pub(super) opened: Option<Instant>,
    pub(super) closing_seen: bool,
    pub(super) focused: bool,
}

thread_local! {
    pub(super) static STATES: RefCell<HashMap<ElementId, State>> = RefCell::new(HashMap::new());
}

pub(super) struct Grab;
impl Render for Grab {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}
