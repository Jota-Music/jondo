use gpui::prelude::FluentBuilder as _;
use gpui::{AnyElement, IntoElement, ParentElement, Styled, div};

use crate::shared::ui::views::components::{button, text};
use crate::shared::ui::views::design::icons;
use crate::shared::ui::views::design::theme;

// The room status sits in the middle of a cover, so its size follows the card:
// the icon is what a Jota tile shows when there is no artwork to paint.
#[allow(clippy::too_many_arguments)]
pub(super) fn status_badge(
    info: &crate::music::rooms::RoomStatus,
    live: bool,
    connecting: bool,
    members: usize,
    pending: bool,
    needs_password: bool,
    known: bool,
    size: f32,
) -> AnyElement {
    if connecting {
        return button::spinner(24.0, theme::AMBER).into_any_element();
    }

    if live {
        return tile(icons::WIFI, members, theme::GREEN, needs_password, size);
    }

    if pending {
        return centered(icons::POWER_OFF, theme::MUTED, size);
    }

    if !known {
        return centered(icons::WIFI_OFF, theme::AMBER, size);
    }

    if !info.active {
        return centered(icons::POWER_OFF, theme::MUTED, size);
    }

    // A room with members but no host is winding down, so it warns instead of
    // showing the usual live green.
    let (icon, tint) = if info.has_host {
        (icons::WIFI, theme::GREEN)
    } else {
        (icons::TRIANGLE_ALERT, theme::AMBER)
    };

    tile(icon, members, tint, needs_password, size)
}

fn tile(
    icon: icons::Icon,
    members: usize,
    tint: gpui::Rgba,
    needs_password: bool,
    size: f32,
) -> AnyElement {
    div()
        .relative()
        .flex()
        .flex_col()
        .items_center()
        .gap(theme::unit(2.0))
        .child(icons::icon(icon, size, tint))
        .child(text::numeric(
            members.to_string(),
            (size * 0.5).max(10.0),
            tint,
        ))
        .when(needs_password, |this| {
            this.child(
                div()
                    .absolute()
                    .bottom(theme::unit(0.0))
                    .right(theme::unit(-(size * 0.22)))
                    .child(icons::icon(
                        icons::LOCK,
                        (size * 0.45).clamp(10.0, 22.0),
                        theme::AMBER,
                    )),
            )
        })
        .into_any_element()
}

fn centered(icon: icons::Icon, tint: gpui::Rgba, size: f32) -> AnyElement {
    div()
        .flex()
        .items_center()
        .justify_center()
        .child(icons::icon(icon, size, tint))
        .into_any_element()
}
