use gpui::prelude::FluentBuilder as _;
use gpui::{ClickEvent, Context, Div, ParentElement, Styled, Window, div};

use crate::shared::i18n::t;
use crate::shared::ui::views::components::{
    button, field,
    select::{self, Select},
};

use crate::shared::ui::views::design::icons;
use crate::shared::ui::views::design::theme;
use crate::shell::app::Jondo;
use crate::shell::views::menu::{Item, Menu, toggle};
use crate::shell::views::page::Click;

/// The search, sort and reload row a list page owns.
pub struct Toolbar {
    pub id: &'static str,
    pub field: &'static str,
    pub placeholder: &'static str,
    pub query: String,
    pub refreshing: bool,
    pub origin: &'static str,
    pub order: String,
    pub items: Vec<Item>,
    pub clear: Click,
    pub reload: Click,
}

pub fn show(
    app: &mut Jondo,
    window: &mut Window,
    cx: &mut Context<Jondo>,
    toolbar: Toolbar,
) -> Div {
    let Toolbar {
        id,
        field: key,
        placeholder,
        query,
        refreshing,
        origin,
        order,
        items,
        clear,
        reload,
    } = toolbar;
    let state = app.field(key, &query, placeholder, window, cx);
    let open = app
        .menu
        .as_ref()
        .is_some_and(|menu| menu.from(&origin.into()));
    div()
        .flex()
        .items_center()
        .gap(theme::unit(8.0))
        .child(
            div()
                .flex_1()
                .h(theme::unit(40.0))
                .flex()
                .items_center()
                .rounded(theme::unit(6.0))
                .border_1()
                .border_color(theme::BORDER)
                .bg(theme::SURFACE)
                .px(theme::unit(12.0))
                .child(icons::icon(icons::SEARCH, 16.0, theme::FAINT))
                .child(div().w(theme::unit(8.0)))
                .child(field::input(&state))
                .when(!query.is_empty(), |this| {
                    this.child(button::icon_flat(
                        format!("{id}-clear"),
                        icons::X,
                        14.0,
                        theme::FAINT,
                        24.0,
                        true,
                        clear,
                    ))
                }),
        )
        .child(select::show(
            Select {
                id: origin,
                title: t("Sort by"),
                label: order,
                open,
                icon: Some(icons::ARROW_UP_DOWN),
                chevron: false,
                full: false,
                radius: 6.0,
                border: theme::BORDER,
            },
            cx.listener(move |app, event: &ClickEvent, _, cx| {
                toggle(
                    app,
                    origin,
                    Menu::choice(event.position(), t("Sort by"), items.clone()),
                );
                app.notify_shell(cx);
            }),
        ))
        .child(button::reload(format!("{id}-refresh"), refreshing, reload))
}
