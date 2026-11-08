use crate::shared::ui::views::design::theme;

use gpui::prelude::FluentBuilder as _;
use gpui::{
    App, Bounds, CursorStyle, Element, ElementId, GlobalElementId, Hitbox, HitboxBehavior,
    InspectorElementId, IntoElement, LayoutId, MouseDownEvent, MouseUpEvent, ParentElement, Pixels,
    Position, Style, Styled, Window, div, relative,
};

use gpui_kit::base::{Scrollbar, ScrollbarHandle, ScrollbarMode, ScrollbarThumbStyle};

const GUTTER: f32 = 15.0;
const THUMB: f32 = 8.0;
const INSET: f32 = 3.5;
const RADIUS: f32 = 4.0;
const MIN: f32 = 40.0;

fn shape(style: ScrollbarThumbStyle) -> ScrollbarThumbStyle {
    style
        .width(theme::unit(THUMB))
        .inset(theme::unit(INSET))
        .radius(theme::unit(RADIUS))
        .min_length(theme::unit(MIN))
}

struct Grab;

impl IntoElement for Grab {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for Grab {
    type RequestLayoutState = ();
    type PrepaintState = Hitbox;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style {
            position: Position::Absolute,
            ..Default::default()
        };
        style.inset.top = theme::unit(0.0).into();
        style.inset.left = theme::unit(0.0).into();
        style.size.width = relative(1.0).into();
        style.size.height = relative(1.0).into();
        (window.request_layout(style, None, cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        window: &mut Window,
        _: &mut App,
    ) -> Self::PrepaintState {
        window.insert_hitbox(bounds, HitboxBehavior::Normal)
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        hitbox: &mut Self::PrepaintState,
        window: &mut Window,
        _: &mut App,
    ) {
        let dragging = window.captured_hitbox() == Some(hitbox.id);
        window.set_cursor_style(
            if dragging {
                CursorStyle::ClosedHand
            } else {
                CursorStyle::OpenHand
            },
            hitbox,
        );

        let id = hitbox.id;
        window.on_mouse_event(move |_: &MouseDownEvent, phase, window, _| {
            if phase.bubble() && id.is_hovered(window) {
                window.capture_pointer(id);
                window.refresh();
            }
        });

        window.on_mouse_event(move |_: &MouseUpEvent, phase, window, _| {
            if phase.bubble() && window.captured_hitbox() == Some(id) {
                window.refresh();
            }
        });
    }
}

pub fn bar<H: ScrollbarHandle + Clone>(handle: &H) -> impl IntoElement {
    let overflow = handle.content_size().height > handle.viewport_bounds().size.height;
    div()
        .absolute()
        .top(theme::unit(0.0))
        .bottom(theme::unit(0.0))
        .right(theme::unit(0.0))
        .w(theme::unit(GUTTER))
        .child(
            Scrollbar::vertical(handle)
                .mode(ScrollbarMode::Always)
                .viewport_from_layout()
                .styles(|styles| {
                    styles
                        .track(|track| track.width(theme::unit(GUTTER)))
                        .thumb(shape)
                        .thumb_hover(shape)
                        .thumb_active(shape)
                }),
        )
        .when(overflow, |this| this.child(Grab))
}
