use gpui::prelude::FluentBuilder as _;
use gpui::{
    Context, Div, ElementId, InteractiveElement, ParentElement, Role, Stateful,
    StatefulInteractiveElement, Styled, div,
};

use crate::shared::ui::views::components::{hover, text};
use crate::shared::ui::views::design::icons;
use crate::shared::ui::views::design::theme;
use crate::shell::app::Jondo;

use super::execute::execute;
use super::{CHOICE_ROW, Item, ROW};

pub(super) fn context_row(
    cx: &mut Context<Jondo>,
    stamp: u64,
    index: usize,
    item: &Item,
    active: bool,
) -> Stateful<Div> {
    let id = ElementId::Name(format!("menu-{stamp}-{index}").into());
    let label = item.label.clone();
    let icon = item.icon;
    let danger = item.danger;
    let command = item.command.clone();
    let fade = hover::value(&id, std::time::Instant::now());
    let lit = if active { 1.0 } else { fade };
    let background = if danger {
        theme::alpha(theme::rgb(0x450a0a), 0.4 * lit)
    } else {
        theme::alpha(theme::ZINC_800, 0.8 * lit)
    };

    let color = if danger {
        theme::DANGER
    } else {
        theme::mix(theme::ZINC_300, theme::rgb(0xffffff), lit)
    };

    div()
        .id(id.clone())
        .h(theme::unit(ROW))
        .w_full()
        .flex()
        .flex_none()
        .items_center()
        .gap(theme::unit(12.0))
        .px(theme::unit(16.0))
        .cursor_pointer()
        .when(lit > 0.0, |row| row.bg(background))
        .when_some(icon, |row, icon| {
            row.child(icons::icon(
                icon,
                16.0,
                if danger { theme::DANGER } else { theme::FAINT },
            ))
        })
        .child(text::text(label.clone(), 14.0, color))
        .role(Role::MenuItem)
        .aria_label(label)
        .aria_position_in_set(index + 1)
        .on_hover({
            let id = id.clone();
            move |on: &bool, _, cx| {
                hover::set(&id, *on, std::time::Instant::now());
                cx.refresh_windows();
            }
        })
        .on_click(cx.listener(move |app, _: &gpui::ClickEvent, window, cx| {
            app.close_menu();
            execute(command.clone(), app, window, cx);
            cx.notify();
        }))
}

pub(super) fn choice_row(
    cx: &mut Context<Jondo>,
    stamp: u64,
    index: usize,
    item: &Item,
    active: bool,
) -> Stateful<Div> {
    let id = ElementId::Name(format!("menu-choice-{stamp}-{index}").into());
    let label = item.label.clone();
    let checked = item.checked;
    let command = item.command.clone();
    let fade = hover::value(&id, std::time::Instant::now());
    let lit = if checked || active { 1.0 } else { fade };
    div()
        .id(id.clone())
        .h(theme::unit(CHOICE_ROW))
        .w_full()
        .flex()
        .flex_none()
        .items_center()
        .gap(theme::unit(8.0))
        .px(theme::unit(10.0))
        .rounded(theme::unit(7.0))
        .cursor_pointer()
        .when(lit > 0.0, |row| row.bg(theme::alpha(theme::ZINC_800, lit)))
        .child(
            text::text(
                label.clone(),
                13.0,
                if checked || active {
                    theme::TEXT
                } else {
                    theme::mix(theme::MUTED, theme::TEXT, fade)
                },
            )
            .flex_1(),
        )
        .child(
            div()
                .w(theme::unit(16.0))
                .h(theme::unit(16.0))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .when(checked, |this| {
                    this.child(icons::icon(icons::CHECK, 16.0, theme::TEXT))
                }),
        )
        .role(Role::MenuItemRadio)
        .aria_label(label)
        .aria_selected(checked)
        .aria_position_in_set(index + 1)
        .on_hover({
            let id = id.clone();
            move |on: &bool, _, cx| {
                hover::set(&id, *on, std::time::Instant::now());
                cx.refresh_windows();
            }
        })
        .on_click(cx.listener(move |app, _: &gpui::ClickEvent, window, cx| {
            app.close_menu();
            execute(command.clone(), app, window, cx);
            cx.notify();
        }))
}
