use gpui::{
    Context, IntoElement, ParentElement, Rgba, StatefulInteractiveElement, Styled, Window, div,
};

use gpui_kit::component::tooltip::Tooltip;

use crate::shared::i18n::t;
use crate::shared::ui::views::components::{button, hover, progress, text};
use crate::shared::ui::views::design::icons::{self, Icon};
use crate::shared::ui::views::design::theme;
use crate::shell::app::{Drag, Jondo};

pub(super) fn preview_bar(app: &mut Jondo, cx: &mut Context<Jondo>) -> impl IntoElement {
    let pending = app.preview.pending;
    let playing = app.preview.playing;
    let duration = app.preview.duration;
    let accent = app.accent;
    let fraction = if duration > 0.0 {
        (app.preview.progress / duration) as f32
    } else {
        0.0
    };

    let toggle = cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
        app.preview.toggle();
        cx.notify();
    });
    let entity = cx.entity();
    let seek = move |value: f32, commit: bool, _: &mut Window, cx: &mut gpui::App| {
        entity.update(cx, |app, cx| {
            app.ui.drag = if commit { None } else { Some(Drag::Preview) };
            app.preview.seek(value as f64 * duration);
            cx.notify();
        });
    };

    let stop = cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
        let preview = &mut app.preview;
        app.player.update(cx, |player, _| preview.stop(player));
        cx.notify();
    });
    div()
        .flex_none()
        .flex()
        .items_center()
        .gap(theme::unit(8.0))
        .border_b_1()
        .border_color(theme::BORDER)
        .px(theme::unit(12.0))
        .py(theme::unit(8.0))
        .child(
            div()
                .w(theme::unit(32.0))
                .h(theme::unit(32.0))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .child(if pending {
                    button::spinner(16.0, theme::ZINC_300).into_any_element()
                } else {
                    button::icon_button(
                        "youtube-preview",
                        if playing { icons::PAUSE } else { icons::PLAY },
                        16.0,
                        theme::ZINC_300,
                        32.0,
                        true,
                        toggle,
                    )
                    .into_any_element()
                }),
        )
        .child(text::numeric(
            crate::shared::kernel::app::format::time(app.preview.progress),
            12.0,
            theme::FAINT,
        ))
        .child(
            div()
                .flex_1()
                .min_w(theme::unit(0.0))
                .child(progress::Bar::new(
                    fraction,
                    None,
                    progress::HEIGHT,
                    accent,
                    theme::ZINC_800,
                    6.0,
                    !pending && duration > 0.0,
                    app.ui.drag == Some(Drag::Preview),
                    seek,
                )),
        )
        .child(text::numeric(
            crate::shared::kernel::app::format::time(duration),
            12.0,
            theme::FAINT,
        ))
        .child(crate::music::player::views::volume_control(
            app,
            cx,
            96.0,
            true,
            Drag::PreviewVolume,
        ))
        .child(soft_button(
            "youtube-preview-stop",
            icons::X,
            t("Stop preview"),
            theme::FAINT,
            stop,
        ))
}

fn soft_button(
    id: &'static str,
    icon: Icon,
    label: &'static str,
    color: Rgba,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    hover::hover(
        id,
        true,
        move |fade| {
            div()
                .w(theme::unit(32.0))
                .h(theme::unit(32.0))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .rounded(theme::unit(6.0))
                .cursor_pointer()
                .bg(theme::alpha(theme::ZINC_800, 0.8 * fade))
                .child(icons::icon(
                    icon,
                    16.0,
                    theme::mix(color, theme::TEXT, fade),
                ))
        },
        on_click,
    )
    .tooltip(move |window, cx| Tooltip::new(label).build(window, cx))
}
