use std::sync::Arc;

use gpui::{Div, FontFeatures, ParentElement, Rgba, SharedString, Styled, div, relative};

use crate::shared::ui::views::design::theme;

/// A single line of body text at the application type scale.
pub fn text(value: impl Into<SharedString>, size: f32, color: Rgba) -> Div {
    div()
        .text_size(theme::unit(size))
        .text_color(color)
        .font_family(theme::FAMILY)
        .line_height(relative(1.2))
        .child(value.into())
}

/// A label whose digits share one advance, so the value never reflows.
pub fn numeric(value: impl Into<SharedString>, size: f32, color: Rgba) -> Div {
    text(value, size, color)
        .font_features(tabular())
        .flex_shrink_0()
}

fn tabular() -> FontFeatures {
    static FEATURES: std::sync::OnceLock<FontFeatures> = std::sync::OnceLock::new();
    FEATURES
        .get_or_init(|| FontFeatures(Arc::new(vec![("tnum".into(), 1)])))
        .clone()
}

pub fn weighted(
    value: impl Into<SharedString>,
    size: f32,
    color: Rgba,
    weight: theme::Weight,
) -> Div {
    text(value, size, color).font_weight(weight.value())
}

/// A body line that shrinks and ends in an ellipsis instead of wrapping.
pub fn truncated(
    value: impl Into<SharedString>,
    size: f32,
    color: Rgba,
    weight: theme::Weight,
) -> Div {
    weighted(value, size, color, weight)
        .min_w(theme::unit(0.0))
        .truncate()
}
