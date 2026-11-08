use gpui::SharedString;

use crate::shared::ui::views::design::icons::Icon;

use super::Command;

#[derive(Clone)]
pub struct Item {
    pub icon: Option<Icon>,
    pub label: SharedString,
    pub danger: bool,
    pub checked: bool,
    pub command: Command,
}

impl Item {
    pub fn new(icon: Icon, label: impl Into<SharedString>, command: Command) -> Self {
        Self {
            icon: Some(icon),
            label: label.into(),
            danger: false,
            checked: false,
            command,
        }
    }

    pub fn danger(icon: Icon, label: impl Into<SharedString>, command: Command) -> Self {
        Self {
            icon: Some(icon),
            label: label.into(),
            danger: true,
            checked: false,
            command,
        }
    }

    pub fn choice(label: impl Into<SharedString>, checked: bool, command: Command) -> Self {
        Self {
            icon: None,
            label: label.into(),
            danger: false,
            checked,
            command,
        }
    }
}
