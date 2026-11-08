use gpui::prelude::FluentBuilder as _;
use gpui::{
    Context, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement, Styled,
    div,
};

use crate::shared::ui::views::components::hover;
use crate::shared::ui::views::design::icons;
use crate::shared::ui::views::design::theme;
use crate::shell::app::Jondo;

pub(super) fn tab_button(
    cx: &mut Context<Jondo>,
    id: &'static str,
    icon: icons::Icon,
    active: bool,
    tab: &'static str,
) -> impl IntoElement {
    let element: gpui::ElementId = id.into();
    let fade = hover::value(&element, std::time::Instant::now());
    div()
        .id(element.clone())
        .w(theme::unit(32.0))
        .h(theme::unit(32.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(theme::unit(6.0))
        .cursor_pointer()
        .when(active, |this| this.bg(theme::ZINC_800))
        .on_hover(move |on: &bool, _, cx| {
            hover::set(&element, *on, std::time::Instant::now());
            cx.refresh_windows();
        })
        .on_click(cx.listener(move |app, _: &gpui::ClickEvent, _, cx| {
            app.ui.channel_tab = tab.to_owned();
            cx.notify();
        }))
        .child(icons::icon(
            icon,
            16.0,
            if active {
                theme::rgb(0xffffff)
            } else {
                theme::mix(theme::FAINT, theme::ZINC_300, fade)
            },
        ))
}
