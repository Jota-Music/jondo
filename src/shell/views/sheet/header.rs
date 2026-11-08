use gpui::{AnyElement, Div, ParentElement, Styled, div};

use crate::shared::ui::views::components::text;
use crate::shared::ui::views::design::icons;
use crate::shared::ui::views::design::theme;

/// The title row every sheet shares: an optional icon, an optional heading and
/// subtitle, and a detail that sits next to them.
pub struct Header {
    pub title: Option<gpui::SharedString>,
    pub subtitle: Option<gpui::SharedString>,
    pub icon: Option<(icons::Icon, gpui::Rgba)>,
    pub size: f32,
    pub detail: Option<AnyElement>,
}

pub fn header(header: Header) -> Div {
    let named = header.title.is_some() || header.subtitle.is_some();
    let mut title = div().flex().flex_col().min_w(theme::unit(0.0));
    if let Some(text) = header.title {
        title = title.child(text::weighted(
            text,
            header.size,
            theme::TEXT,
            theme::Weight::SemiBold,
        ));
    }

    if let Some(subtitle) = header.subtitle {
        title = title.child(div().mt(theme::unit(2.0)).child(text::truncated(
            subtitle,
            12.0,
            theme::FAINT,
            theme::Weight::Regular,
        )));
    }

    let mut leading = div()
        .flex()
        .items_center()
        .gap(theme::unit(8.0))
        .min_w(theme::unit(0.0))
        .min_h(theme::unit(32.0))
        .children(
            header
                .icon
                .map(|(icon, color)| icons::icon(icon, 22.0, color)),
        );
    if named {
        leading = leading.child(title);
    }

    leading = leading.children(header.detail);
    div()
        .flex()
        .items_center()
        .gap(theme::unit(8.0))
        .pl(theme::unit(22.0))
        .pr(theme::unit(56.0))
        .py(theme::unit(12.0))
        .child(leading)
}
