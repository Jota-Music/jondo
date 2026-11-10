use std::time::{Duration, Instant};

use gpui::{
    Context, ElementId, InteractiveElement, IntoElement, MouseButton, ParentElement, Pixels, Point,
    SharedString, StatefulInteractiveElement, Styled, Window, div,
};

use crate::shared::ui::views::components::text;
use crate::shared::ui::views::design::theme;
use crate::shell::app::Jondo;
use crate::shell::views::layouts;

use super::execute::execute;
use super::item::Item;
use super::rows::{choice_row, context_row};
use super::sheet::sheet;
use super::{CHOICE_ROW, CHOICE_WIDTH, EXIT, GAP, HEADER, OPEN, ROW, WIDTH};

#[derive(Clone)]
pub struct Menu {
    pub stamp: u64,
    pub at: Point<Pixels>,
    pub header: Option<SharedString>,
    pub context: bool,
    pub bottom: f32,
    pub index: Option<usize>,
    pub origin: Option<ElementId>,
    pub opened: Instant,
    pub items: Vec<Item>,
}

impl Menu {
    pub fn context(
        at: Point<Pixels>,
        header: Option<SharedString>,
        items: Vec<Item>,
        bottom: f32,
    ) -> Self {
        let mut items = items;
        items.sort_by_key(|item| item.danger);
        Self {
            stamp: next(),
            at,
            header,
            context: true,
            bottom,
            index: None,
            origin: None,
            opened: Instant::now(),
            items,
        }
    }

    pub fn choice(at: Point<Pixels>, header: impl Into<SharedString>, items: Vec<Item>) -> Self {
        Self {
            stamp: next(),
            at,
            header: Some(header.into()),
            context: false,
            bottom: 0.0,
            index: items.iter().position(|item| item.checked),
            origin: None,
            opened: Instant::now(),
            items,
        }
    }

    /// The id of the control that opened this menu, so it can keep its open
    /// surface while the list is visible.
    pub fn origin(mut self, origin: impl Into<ElementId>) -> Self {
        self.origin = Some(origin.into());
        self
    }

    /// Whether `origin` is the control that opened this menu.
    pub fn from(&self, origin: &ElementId) -> bool {
        self.origin.as_ref() == Some(origin)
    }

    /// Moves the highlighted row the way a native menu does, without wrapping,
    /// so the first and last rows stop under the arrow keys.
    pub fn step(&mut self, delta: isize) {
        let last = self.items.len().saturating_sub(1) as isize;
        let next = match self.index {
            Some(index) => (index as isize + delta).clamp(0, last),
            None if delta > 0 => 0,
            None => last,
        };

        self.index = Some(next as usize);
    }
}

/// Opens `menu` under the control `origin`, or closes the open menu when that
/// same control already opened it, so a trigger works as a toggle. A menu that
/// is already sliding out opens again instead of ignoring the press.
pub fn toggle(app: &mut Jondo, origin: impl Into<ElementId>, menu: Menu) -> bool {
    let origin = origin.into();
    if app.menu_closing.is_none() && app.menu.as_ref().is_some_and(|open| open.from(&origin)) {
        app.close_menu();
        return false;
    }

    app.menu = Some(menu.origin(origin));
    app.menu_closing = None;
    true
}

/// Keyboard control for the open menu: arrows move the highlight and Enter
/// activates it. Returns whether the key belonged to the menu.
pub fn keys(app: &mut Jondo, key: &str, window: &mut Window, cx: &mut Context<Jondo>) -> bool {
    let command = {
        let Some(menu) = app.menu.as_mut() else {
            return false;
        };

        match key {
            "down" => {
                menu.step(1);
                None
            }
            "up" => {
                menu.step(-1);
                None
            }
            "enter" => menu
                .index
                .and_then(|index| menu.items.get(index))
                .map(|item| item.command.clone()),
            _ => return false,
        }
    };

    if let Some(command) = command {
        app.close_menu();
        execute(command, app, window, cx);
    }

    cx.notify();
    true
}

pub fn show(app: &mut Jondo, window: &mut Window, cx: &mut Context<Jondo>) -> gpui::AnyElement {
    if let Some((stamp, at)) = app.menu_closing {
        if at.elapsed() >= crate::shell::views::sheet::DURATION {
            app.menu = None;
            app.menu_closing = None;
        } else if app.menu.as_ref().is_some_and(|menu| menu.stamp != stamp) {
            app.menu_closing = None;
        }
    }

    let Some(menu) = app.menu.clone() else {
        return div().into_any_element();
    };

    let closing = app
        .menu_closing
        .filter(|(stamp, _)| *stamp == menu.stamp)
        .map(|(_, at)| at);
    // A context menu is a pointer-sized popover, so a narrow desktop window
    // must not turn it into a modal sheet. Android has no right button: its
    // context menu is opened by a long press and is presented as a bottom
    // sheet, like every other action list on the phone.
    let as_sheet = presents_as_sheet(menu.context, mobile(window), cfg!(target_os = "android"));
    if as_sheet {
        return div()
            .absolute()
            .inset_0()
            .bottom(theme::unit(layouts::bottom(window, true)))
            .overflow_hidden()
            .child(sheet(app, window, cx, menu, closing))
            .into_any_element();
    }

    let now = Instant::now();
    let fade = match closing {
        Some(at) => (1.0 - progress(at, EXIT, now)) * progress(menu.opened, OPEN, at),
        None => progress(menu.opened, OPEN, now),
    };

    if fade < 1.0 {
        crate::shell::app::motion::ask(window);
    }

    let viewport = window.viewport_size();
    let width = if menu.context { WIDTH } else { CHOICE_WIDTH };
    let row = if menu.context { ROW } else { CHOICE_ROW };
    let padding = if menu.context { 8.0 } else { 10.0 };
    let header = if menu.context && menu.header.is_some() {
        HEADER
    } else {
        0.0
    };

    let height = padding + header + row * menu.items.len() as f32;
    let bottom_bar = if viewport.width.as_f32() < 768.0 {
        56.0
    } else {
        0.0
    };

    let x = menu
        .at
        .x
        .as_f32()
        .min(viewport.width.as_f32() - width - GAP)
        .max(GAP);
    let y = menu
        .at
        .y
        .as_f32()
        .min(viewport.height.as_f32() - bottom_bar - menu.bottom - height - GAP - 4.0)
        .max(GAP);

    let mut panel = div()
        .id(gpui::ElementId::Name(
            format!("menu-panel-{}", menu.stamp).into(),
        ))
        .absolute()
        .left(theme::unit(x))
        .top(theme::unit(y + 4.0 + 4.0 * (1.0 - fade)))
        .w(theme::unit(width))
        .rounded(theme::unit(10.0))
        .bg(theme::SURFACE)
        .border_1()
        .border_color(theme::BORDER)
        .opacity(fade)
        .shadow_xl()
        .role(gpui::Role::Menu)
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation());

    if menu.context {
        panel = panel.py(theme::unit(4.0));
        if let Some(label) = &menu.header {
            panel = panel
                .aria_label(label.clone())
                .child(
                    div()
                        .px(theme::unit(16.0))
                        .pt(theme::unit(8.0))
                        .pb(theme::unit(4.0))
                        .child(text::weighted(
                            label.clone(),
                            12.0,
                            theme::FAINT,
                            theme::Weight::Medium,
                        )),
                )
                .child(
                    div()
                        .mx(theme::unit(8.0))
                        .mb(theme::unit(4.0))
                        .h(theme::unit(1.0))
                        .bg(theme::BORDER),
                );
        }
    } else {
        panel = panel.p(theme::unit(5.0));
    }

    for (index, item) in menu.items.iter().enumerate() {
        let active = menu.index == Some(index);
        panel = panel.child(if menu.context {
            context_row(cx, menu.stamp, index, item, active)
        } else {
            choice_row(cx, menu.stamp, index, item, active)
        });
    }

    let dismiss = cx.listener(|app, _: &gpui::MouseDownEvent, _, cx| {
        app.close_menu();
        app.notify_shell(cx);
    });

    div()
        .absolute()
        .inset_0()
        .occlude()
        .on_mouse_down(MouseButton::Left, dismiss)
        .on_mouse_down(
            MouseButton::Right,
            cx.listener(|app, _: &gpui::MouseDownEvent, _, cx| {
                app.close_menu();
                app.notify_shell(cx);
            }),
        )
        .child(panel)
        .into_any_element()
}

pub(super) fn mobile(window: &Window) -> bool {
    window.viewport_size().width < theme::unit(768.0)
}

/// Whether a menu is a bottom sheet instead of a popover anchored to the
/// pointer. Choices use the sheet on narrow viewports; context menus only on
/// Android, where there is no right button and every action list is modal.
pub(super) fn presents_as_sheet(context: bool, mobile: bool, android: bool) -> bool {
    (!context && mobile) || (context && android)
}

fn progress(at: Instant, duration: Duration, now: Instant) -> f32 {
    (now.duration_since(at).as_secs_f32() / duration.as_secs_f32()).clamp(0.0, 1.0)
}

fn next() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static STAMP: AtomicU64 = AtomicU64::new(0);
    STAMP.fetch_add(1, Ordering::Relaxed)
}
