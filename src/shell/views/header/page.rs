use gpui::{AnimationExt, Context, IntoElement, ParentElement, Styled, Window, div};

use crate::shared::ui::views::design::theme;
use crate::shell::app::Jondo;

use super::bar::bar;
use super::search::search_panel;

pub fn show(app: &mut Jondo, window: &mut Window, cx: &mut Context<Jondo>) -> impl IntoElement {
    let narrow = window.viewport_size().width < theme::unit(768.0);
    let icon = if narrow { 20.0 } else { 16.0 };
    let bar = bar(app, window, cx, narrow, icon);
    if let Some(at) = app.ui.search_closing
        && at.elapsed() > std::time::Duration::from_millis(200)
    {
        app.ui.search_closing = None;
        app.ui.search_open = false;
    }

    let panel = if app.ui.search_open || app.ui.search_closing.is_some() {
        let opacity = app
            .ui
            .search_closing
            .map(|at| 1.0 - (at.elapsed().as_secs_f32() / 0.2).clamp(0.0, 1.0))
            .unwrap_or(1.0);
        let full = if narrow { 110.0 } else { 56.0 };
        if app.ui.search_closing.is_some() {
            div()
                .opacity(opacity)
                .max_h(theme::unit(full * opacity))
                .overflow_hidden()
                .child(search_panel(app, window, cx, narrow, icon))
                .into_any_element()
        } else {
            div()
                .overflow_hidden()
                .child(search_panel(app, window, cx, narrow, icon))
                .with_animation(
                    "search-panel",
                    gpui::Animation::new(std::time::Duration::from_millis(200)),
                    move |panel, delta| panel.max_h(theme::unit(full * delta)).opacity(delta),
                )
                .into_any_element()
        }
    } else {
        div().w_full().into_any_element()
    };

    div()
        .w_full()
        .flex_none()
        .flex()
        .flex_col()
        .bg(theme::SURFACE)
        .child(bar)
        .child(panel)
        .child(div().w_full().h(theme::unit(1.0)).bg(theme::BORDER))
}
