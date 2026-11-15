use gpui::SharedString;

use crate::shared::ui::views::design::icons::Icon;

use super::Command;

#[derive(Clone)]
pub struct Item {
    pub icon: Option<Icon>,
    pub label: SharedString,
    pub danger: bool,
    pub checked: bool,
    /// The toggle is already set, so its icon reads as pressed: the same red
    /// filled heart the page header shows.
    pub on: bool,
    pub command: Command,
}

impl Item {
    pub fn new(icon: Icon, label: impl Into<SharedString>, command: Command) -> Self {
        Self {
            icon: Some(icon),
            label: label.into(),
            danger: false,
            checked: false,
            on: false,
            command,
        }
    }

    pub fn danger(icon: Icon, label: impl Into<SharedString>, command: Command) -> Self {
        Self {
            icon: Some(icon),
            label: label.into(),
            danger: true,
            checked: false,
            on: false,
            command,
        }
    }

    pub fn choice(label: impl Into<SharedString>, checked: bool, command: Command) -> Self {
        Self {
            icon: None,
            label: label.into(),
            danger: false,
            checked,
            on: false,
            command,
        }
    }

    pub fn on(mut self, on: bool) -> Self {
        self.on = on;
        self
    }
}
