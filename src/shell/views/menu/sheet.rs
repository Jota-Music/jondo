use std::time::Instant;

use gpui::prelude::FluentBuilder as _;
use gpui::{
    Context, Div, ElementId, InteractiveElement, IntoElement, MouseButton, ParentElement, Rgba,
    Role, Stateful, StatefulInteractiveElement, Styled, Toggled, Window, div,
};

use crate::shared::i18n::t;
use crate::shared::ui::views::components::paint::{self, Scope};
use crate::shared::ui::views::components::{hover, text};
use crate::shared::ui::views::design::icons;
use crate::shared::ui::views::design::theme;
use crate::shell::app::Jondo;
use crate::shell::views::sheet as sheet_ui;

use super::execute::execute;
use super::{Item, Menu};

pub(super) fn sheet(
    app: &mut Jondo,
    window: &mut Window,
    cx: &mut Context<Jondo>,
    menu: Menu,
    closing: Option<Instant>,
) -> gpui::AnyElement {
    let has_song = app.player.read(cx).current().is_some();
    let menu_scroll = app.pane("menu-context");
    let panel = if menu.context {
        context(&menu, &menu_scroll, cx)
    } else {
        select(&menu, app.accent, app.binary, cx)
    }
    .into_any_element();

    let sheet = sheet_ui::Sheet::new(
        "menu-sheet",
        closing,
        |app, _, cx| {
            app.close_menu();
            app.notify_shell(cx);
        },
        panel,
    )
    .mobile(true)
    .bar(has_song);
    let sheet = if menu.context {
        sheet.at_top(move || menu_scroll.offset().y >= gpui::px(0.0))
    } else {
        sheet
    };
    sheet.show(window, cx)
}

/// A single-choice list, laid out like a native dialog: a title, one row per
/// option, and a checkbox on the trailing edge showing what is selected.
fn select(menu: &Menu, accent: Rgba, binary: Rgba, cx: &mut Context<Jondo>) -> Stateful<Div> {
    let mut panel = div()
        .id("menu-select")
        .flex()
        .flex_col()
        .pb(theme::unit(12.0))
        .role(Role::Dialog)
        .aria_label(
            menu.header
                .clone()
                .unwrap_or_else(|| t("Select").to_owned().into()),
        );

    panel = panel.child(title(menu, 20.0, "Select"));

    for (index, item) in menu.items.iter().enumerate() {
        panel = panel.child(select_row(
            cx,
            menu.stamp,
            index,
            item,
            menu.index == Some(index),
            accent,
            binary,
        ));
    }

    panel
}

/// The action list a long press opens on Android: the same items the desktop
/// popover shows, sized for a finger and pinned to the bottom edge.
fn context(menu: &Menu, scroll: &gpui::ScrollHandle, cx: &mut Context<Jondo>) -> Stateful<Div> {
    let mut panel = div()
        .id("menu-context")
        .flex()
        .flex_col()
        .flex_1()
        .min_h(theme::unit(0.0))
        .pb(theme::unit(12.0))
        .role(Role::Dialog)
        .aria_label(
            menu.header
                .clone()
                .unwrap_or_else(|| t("More actions").to_owned().into()),
        );

    panel = panel.child(title(menu, 20.0, "More actions"));

    let mut list = div()
        .id("menu-context-list")
        .flex()
        .flex_col()
        .flex_1()
        .min_h(theme::unit(0.0))
        .overflow_y_scroll()
        .track_scroll(scroll);
    for (index, item) in menu.items.iter().enumerate() {
        list = list.child(context_row(cx, menu.stamp, index, item));
    }

    panel.child(list)
}

fn title(menu: &Menu, size: f32, fallback: &'static str) -> Div {
    div()
        .flex()
        .items_center()
        .pl(theme::unit(20.0))
        .pr(theme::unit(48.0))
        .pt(theme::unit(8.0))
        .pb(theme::unit(16.0))
        .child(text::weighted(
            menu.header
                .clone()
                .unwrap_or_else(|| t(fallback).to_owned().into()),
            size,
            theme::TEXT,
            theme::Weight::SemiBold,
        ))
}

fn context_row(cx: &mut Context<Jondo>, stamp: u64, index: usize, item: &Item) -> Stateful<Div> {
    let id = ElementId::Name(format!("menu-context-{stamp}-{index}").into());
    let label = item.label.clone();
    let icon = item.icon;
    let danger = item.danger;
    let command = item.command.clone();
    let color = if danger {
        theme::DANGER
    } else {
        theme::ZINC_300
    };

    div()
        .id(id)
        .w_full()
        .min_h(theme::unit(56.0))
        .flex_none()
        .flex()
        .items_center()
        .gap(theme::unit(16.0))
        .px(theme::unit(20.0))
        .cursor_pointer()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .when_some(icon, |row, icon| {
            row.child(icons::icon(
                icon,
                18.0,
                if danger { theme::DANGER } else { theme::MUTED },
            ))
        })
        .child(text::text(label.clone(), 16.0, color).flex_1())
        .role(Role::MenuItem)
        .aria_label(label)
        .aria_position_in_set(index + 1)
        .on_click(cx.listener(move |app, _: &gpui::ClickEvent, window, cx| {
            app.close_menu();
            execute(command.clone(), app, window, cx);
            cx.notify();
        }))
}

fn select_row(
    cx: &mut Context<Jondo>,
    stamp: u64,
    index: usize,
    item: &Item,
    active: bool,
    accent: Rgba,
    binary: Rgba,
) -> Stateful<Div> {
    let id = ElementId::Name(format!("menu-select-{stamp}-{index}").into());
    let label = item.label.clone();
    let checked = item.checked;
    let command = item.command.clone();
    let fade = hover::value(&id, std::time::Instant::now());
    let lit = if active { 1.0 } else { fade };
    div()
        .id(id.clone())
        .w_full()
        .min_h(theme::unit(56.0))
        .flex_none()
        .flex()
        .items_center()
        .gap(theme::unit(12.0))
        .px(theme::unit(20.0))
        .cursor_pointer()
        .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .when(lit > 0.0, |row| row.bg(theme::alpha(theme::ZINC_800, lit)))
        .child(
            text::text(
                label.clone(),
                16.0,
                if checked {
                    theme::TEXT
                } else {
                    theme::ZINC_300
                },
            )
            .flex_1(),
        )
        .child(checkbox(checked, accent, binary))
        .role(Role::CheckBox)
        .aria_label(label)
        .aria_toggled(if checked {
            Toggled::True
        } else {
            Toggled::False
        })
        .on_hover({
            let id = id.clone();
            move |on: &bool, window, cx| {
                hover::set_in(
                    Scope::Shell,
                    &id,
                    *on,
                    std::time::Instant::now(),
                    window.last_input_was_touch(),
                );
                paint::paint(Scope::Shell, cx);
            }
        })
        .on_click(cx.listener(move |app, _: &gpui::ClickEvent, window, cx| {
            app.close_menu();
            execute(command.clone(), app, window, cx);
            cx.notify();
        }))
}

fn checkbox(checked: bool, accent: Rgba, binary: Rgba) -> Div {
    div()
        .w(theme::unit(22.0))
        .h(theme::unit(22.0))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(theme::unit(5.0))
        .border_1()
        .border_color(if checked { accent } else { theme::ZINC_600 })
        .when(checked, |this| this.bg(accent))
        .when(checked, |this| {
            this.child(icons::icon(icons::CHECK, 14.0, binary))
        })
}
