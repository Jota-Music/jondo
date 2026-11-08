use std::time::Duration;

use gpui::prelude::FluentBuilder as _;
use gpui::{
    Animation, AnimationExt, Context, ElementId, IntoElement, ParentElement, Radians, Styled, div,
};

use crate::shared::ui::views::components::{hover, text};
use crate::shared::ui::views::design::icons;
use crate::shared::ui::views::design::theme;
use crate::shell::app::Jondo;

#[allow(clippy::too_many_arguments)]
pub(super) fn tab_button(
    cx: &mut Context<Jondo>,
    key: &str,
    label: &str,
    active: bool,
    loading: bool,
    icon: icons::Icon,
    count: Option<usize>,
    width: f32,
) -> impl IntoElement {
    let key = key.to_owned();
    let label = label.to_owned();
    let disabled = loading || count == Some(0);
    let spin = format!("artist-tab-spinner-{key}");
    hover::hover(
        format!("artist-tab-{key}"),
        !disabled,
        move |fade| {
            let foreground = if active {
                theme::TEXT
            } else {
                theme::mix(theme::MUTED, theme::rgb(0xffffff), fade)
            };

            div()
                .w(theme::unit(width))
                .h(theme::unit(36.0))
                .flex_none()
                .px(theme::unit(12.0))
                .flex()
                .items_center()
                .gap(theme::unit(8.0))
                .rounded(theme::unit(6.0))
                .border_1()
                .border_color(if active {
                    theme::ZINC_600
                } else {
                    theme::ZINC_800
                })
                .bg(if active {
                    theme::ZINC_900
                } else {
                    theme::SURFACE
                })
                .when(disabled, |this| this.opacity(0.4))
                .child(icons::icon(icon, 14.0, foreground))
                .child(div().flex_1().min_w(theme::unit(0.0)).child(text::text(
                    label.clone(),
                    14.0,
                    foreground,
                )))
                .when(loading, |this| this.child(spinner(&spin)))
                .when_some(count, |this, count| {
                    this.child(text::text(
                        count.to_string(),
                        12.0,
                        if active {
                            theme::FAINT
                        } else {
                            theme::ZINC_600
                        },
                    ))
                })
        },
        cx.listener(move |app, _: &gpui::ClickEvent, _, cx| {
            app.ui.artist_tab = key.clone();
            app.clear_selection();
            cx.notify();
        }),
    )
}

fn spinner(id: &str) -> impl IntoElement {
    icons::icon(icons::LOADER, 12.0, theme::FAINT).with_animation(
        ElementId::Name(id.to_owned().into()),
        Animation::new(Duration::from_secs(1))
            .repeat()
            .with_max_fps(60.0),
        |glyph, delta| {
            glyph.with_transformation(gpui::Transformation::rotate(Radians(
                delta * std::f32::consts::TAU,
            )))
        },
    )
}
