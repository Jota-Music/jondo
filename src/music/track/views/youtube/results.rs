use std::time::Instant;

use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, Axis, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Window, div,
};

use gpui_kit::base::ScrollableMask;

use crate::shared::i18n::t;
use crate::shared::music::views::cover;
use crate::shared::ui::views::components::paint::Scope;
use crate::shared::ui::views::components::{hover, loading, scroll, text};
use crate::shared::ui::views::design::icons;
use crate::shared::ui::views::design::theme;
use crate::shared::youtube::Video;
use crate::shell::app::Jondo;

pub(super) fn results(
    app: &mut Jondo,
    window: &Window,
    cx: &mut Context<Jondo>,
    active_id: &str,
    slide: f32,
) -> AnyElement {
    let height = window.viewport_size().height.as_f32() * 0.4;
    let content = if app.editor.query.is_empty() {
        div()
            .py(theme::unit(24.0))
            .child(text::text(t("Type to search YouTube."), 14.0, theme::FAINT))
            .into_any_element()
    } else if app.editor.results.loading {
        loading::spinner().into_any_element()
    } else if app.editor.results.error.is_some() {
        div()
            .child(text::text(
                t("Could not search YouTube."),
                14.0,
                theme::DANGER,
            ))
            .into_any_element()
    } else if app.editor.results.empty() {
        div()
            .child(text::text(t("No videos found."), 14.0, theme::FAINT))
            .into_any_element()
    } else {
        let scroll = app.pane("youtube-results");
        let videos = app.editor.results.list().to_vec();
        let mut list = div()
            .id("youtube-results")
            .size_full()
            .flex()
            .flex_col()
            .overflow_y_scroll()
            .track_scroll(&scroll)
            .pr(theme::unit(15.0));
        for (index, video) in videos.iter().enumerate() {
            list = list.child(video_row(app, cx, index, video, active_id));
        }

        div()
            .relative()
            .size_full()
            .child(list)
            .child(ScrollableMask::new(Axis::Vertical, &scroll))
            .child(scroll::bar(&scroll))
            .into_any_element()
    };

    div()
        .flex_none()
        .w_full()
        .h(theme::unit(height * slide))
        .mt(theme::unit(12.0 * slide - 12.0))
        .rounded(theme::unit(8.0))
        .border_1()
        .border_color(theme::BORDER)
        .bg(theme::SURFACE)
        .overflow_hidden()
        .opacity(slide)
        .child(content)
        .into_any_element()
}

fn video_row(
    app: &mut Jondo,
    cx: &mut Context<Jondo>,
    index: usize,
    video: &Video,
    active_id: &str,
) -> impl IntoElement {
    let id = video.id.clone();
    let active = active_id == id;
    let current = app.preview.video.as_deref() == Some(id.as_str());
    let pending = current && app.preview.pending;
    let playing = current && app.preview.playing;
    let accent = app.accent;
    let duration = video.duration;
    let song = video.song();
    let art = cover::art(
        &mut app.covers,
        song.cover.as_deref().unwrap_or_default(),
        40.0,
        40.0,
        6.0,
    );
    let row_id = gpui::ElementId::Name(format!("youtube-video-{index}").into());
    let fade = hover::value(&row_id, Instant::now());
    let background = if active { 0.6 } else { 0.6 * fade };
    let preview = id.clone();
    let select = id.clone();
    div()
        .id(row_id.clone())
        .relative()
        .w_full()
        .h(theme::unit(64.0))
        .flex_none()
        .flex()
        .items_center()
        .gap(theme::unit(8.0))
        .px(theme::unit(12.0))
        .border_b_1()
        .border_color(theme::ZINC_900)
        .cursor_pointer()
        .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .bg(theme::alpha(theme::ZINC_800, background))
        .on_hover(cx.listener({
            let id = row_id.clone();
            move |app, on: &bool, window, cx| {
                hover::set_in(
                    Scope::Page,
                    &id,
                    *on,
                    Instant::now(),
                    window.last_input_was_touch(),
                );
                app.paint_page(cx);
            }
        }))
        .on_click(
            cx.listener(move |app, event: &gpui::ClickEvent, window, cx| {
                let double = matches!(
                    event,
                    gpui::ClickEvent::Mouse(mouse) if mouse.down.click_count >= 2
                );
                app.editor.link = select.clone();
                app.editor.resolving = false;
                app.editor.link_at = None;
                app.set_field("youtube-link", &select, window, cx);
                if double {
                    let preview_state = &mut app.preview;
                    app.player.update(cx, |player, _| {
                        preview_state.start(player, &app.youtube, &app.jobs, &preview)
                    });
                }

                cx.notify();
            }),
        )
        .child(
            div()
                .relative()
                .flex_none()
                .child(art)
                .when(current, |this| {
                    this.child(cover::outline(2.0, 6.0, accent))
                        .child(cover::current(pending, playing, 6.0, 25.0, accent))
                }),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_w(theme::unit(0.0))
                .child(text::truncated(
                    video.title.clone(),
                    14.0,
                    if current { accent } else { theme::TEXT },
                    if current {
                        theme::Weight::Medium
                    } else {
                        theme::Weight::Regular
                    },
                ))
                .child(text::truncated(
                    video.author.clone(),
                    12.0,
                    theme::MUTED,
                    theme::Weight::Regular,
                )),
        )
        .child(
            div()
                .flex_none()
                .flex()
                .items_center()
                .gap(theme::unit(8.0))
                .when(duration > 0, |this| {
                    this.child(text::numeric(
                        crate::shared::kernel::app::format::time(duration as f64),
                        12.0,
                        theme::FAINT,
                    ))
                })
                .when(active, |this| {
                    this.child(icons::icon(icons::CHECK_BOLD, 16.0, theme::TEXT))
                }),
        )
}
