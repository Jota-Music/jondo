use gpui::{AnyElement, Context, IntoElement, Window};

use crate::music::library::views::shelf;
use crate::shell::app::Jondo;

pub fn show(app: &mut Jondo, window: &mut Window, cx: &mut Context<Jondo>) -> AnyElement {
    shelf::following(app, window, cx).into_any_element()
}
