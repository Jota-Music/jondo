use gpui::{
    AnyElement, Context, CursorStyle, InteractiveElement, IntoElement, ParentElement, ResizeEdge,
    Styled, Window, div,
};

use crate::shared::kernel::app::mutex;
use crate::shared::ui::views::design::theme;
use crate::shell::app::Jondo;

/// What the system gesture bar and the phone navigation reserve at the bottom
/// edge. Overlays anchored to the window bottom must subtract it themselves:
/// absolute children are positioned against the border box, not the shell's
/// padded content box.
pub fn bottom(window: &Window, narrow: bool) -> f32 {
    system(window) + if narrow { theme::NAV_HEIGHT } else { 0.0 }
}

/// What the system bars reserve at the bottom edge, without the keyboard. The
/// shell paints this strip with the page background, so a transparent system
/// navigation bar shows the app colour instead of the sheet sliding out behind
/// it.
pub fn system(window: &Window) -> f32 {
    let (_, inset, _, _) = safe_area();
    system_bottom(f32::from(window.viewport_size().width), inset)
}

/// The keyboard is reported as a growing bottom inset, and the shell must not
/// ride up with it: the bars and the page stay where they are, and only the
/// surface that owns the caret lifts. So the shell keeps the system inset seen
/// before the keyboard, which is the smallest one for a given width.
fn system_bottom(width: f32, bottom: f32) -> f32 {
    use std::sync::Mutex;
    static CACHE: Mutex<Option<(f32, f32)>> = Mutex::new(None);
    let mut cache = mutex::lock(&CACHE);
    match *cache {
        Some((cached_width, cached)) if (cached_width - width).abs() < 1.0 => {
            if bottom > 0.0 && bottom < cached {
                *cache = Some((cached_width, bottom));
                bottom
            } else {
                cached
            }
        }
        _ => {
            *cache = Some((width, bottom));
            bottom
        }
    }
}

/// How much of the window the keyboard covers, in logical pixels. Zero when it
/// is closed. Surfaces holding an editable field lift by this.
pub fn keyboard(window: &Window) -> f32 {
    let (_, bottom, _, _) = safe_area();
    let viewport = window.viewport_size();
    (bottom - system_bottom(f32::from(viewport.width), bottom)).max(0.0)
}

pub(super) fn safe_area() -> (f32, f32, f32, f32) {
    #[cfg(target_os = "android")]
    {
        use gpui_mobile::android::jni;
        if let Some(window) = jni::platform().and_then(|platform| platform.primary_window()) {
            let insets = window.safe_area_insets_logical();
            return (insets.top, insets.bottom, insets.left, insets.right);
        }
    }

    (0.0, 0.0, 0.0, 0.0)
}

pub(super) fn resize_bands(cx: &mut Context<Jondo>) -> AnyElement {
    fn band(
        cx: &mut Context<Jondo>,
        edge: ResizeEdge,
        cursor: CursorStyle,
        area: impl FnOnce(gpui::Div) -> gpui::Div,
    ) -> AnyElement {
        area(div().absolute().cursor(cursor).on_mouse_down(
            gpui::MouseButton::Left,
            cx.listener(move |_, _: &gpui::MouseDownEvent, window, _| {
                window.start_window_resize(edge);
            }),
        ))
        .into_any_element()
    }

    div()
        .absolute()
        .inset_0()
        .child(band(
            cx,
            ResizeEdge::Left,
            CursorStyle::ResizeLeftRight,
            |d| {
                d.left(theme::unit(0.0))
                    .top(theme::unit(6.0))
                    .bottom(theme::unit(6.0))
                    .w(theme::unit(6.0))
            },
        ))
        .child(band(
            cx,
            ResizeEdge::Right,
            CursorStyle::ResizeLeftRight,
            |d| {
                d.right(theme::unit(0.0))
                    .top(theme::unit(6.0))
                    .bottom(theme::unit(6.0))
                    .w(theme::unit(6.0))
            },
        ))
        .child(band(cx, ResizeEdge::Top, CursorStyle::ResizeUpDown, |d| {
            d.top(theme::unit(0.0))
                .left(theme::unit(6.0))
                .right(theme::unit(6.0))
                .h(theme::unit(6.0))
        }))
        .child(band(
            cx,
            ResizeEdge::Bottom,
            CursorStyle::ResizeUpDown,
            |d| {
                d.bottom(theme::unit(0.0))
                    .left(theme::unit(6.0))
                    .right(theme::unit(6.0))
                    .h(theme::unit(6.0))
            },
        ))
        .child(band(
            cx,
            ResizeEdge::TopLeft,
            CursorStyle::ResizeUpLeftDownRight,
            |d| {
                d.left(theme::unit(0.0))
                    .top(theme::unit(0.0))
                    .w(theme::unit(6.0))
                    .h(theme::unit(6.0))
            },
        ))
        .child(band(
            cx,
            ResizeEdge::TopRight,
            CursorStyle::ResizeUpRightDownLeft,
            |d| {
                d.right(theme::unit(0.0))
                    .top(theme::unit(0.0))
                    .w(theme::unit(6.0))
                    .h(theme::unit(6.0))
            },
        ))
        .child(band(
            cx,
            ResizeEdge::BottomLeft,
            CursorStyle::ResizeUpRightDownLeft,
            |d| {
                d.left(theme::unit(0.0))
                    .bottom(theme::unit(0.0))
                    .w(theme::unit(6.0))
                    .h(theme::unit(6.0))
            },
        ))
        .child(band(
            cx,
            ResizeEdge::BottomRight,
            CursorStyle::ResizeUpLeftDownRight,
            |d| {
                d.right(theme::unit(0.0))
                    .bottom(theme::unit(0.0))
                    .w(theme::unit(6.0))
                    .h(theme::unit(6.0))
            },
        ))
        .into_any_element()
}
