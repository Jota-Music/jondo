use std::sync::Arc;
use std::time::Duration;

use gpui::{
    Animation, AnimationExt, AnyElement, Div, IntoElement, ObjectFit, ParentElement, RenderImage,
    Rgba, Styled, StyledImage, div, img,
};

use crate::shared::cover::Covers;
use crate::shared::music::domain::Source;
use crate::shared::ui::views::design::icons::{self, Icon};
use crate::shared::ui::views::design::theme;

/// The two 1.2 lines under a cover, their gap and the 8/6 padding around them,
/// in design pixels.
pub const LABEL: f32 = 8.0 + 14.0 * 1.2 + 2.0 + 12.0 * 1.2 + 6.0;

/// The tile a cover with a two-line label occupies: the artwork (with its 6px
/// top inset) plus that strip. Pinning it keeps uniform list rows aligned.
pub fn tile_height(width: f32) -> f32 {
    width - 6.0 + LABEL
}

/// Artwork for a remote URL while it is still on its way. The image cache
/// only answers once the bytes are decoded, so a URL that is not ready yet
/// owes the reader a shimmer rather than a hollow box.
pub fn art(covers: &mut Covers, url: &str, width: f32, height: f32, radius: f32) -> AnyElement {
    if url.is_empty() {
        return missing(width, height, radius);
    }

    if let Some(image) = covers.image(url, width.max(height) * 2.0) {
        return cover(Some(image), width, height, radius);
    }

    if covers.failed(url) {
        return missing(width, height, radius);
    }

    shimmer(width, height, radius).into_any_element()
}

pub fn cover(image: Option<Arc<RenderImage>>, width: f32, height: f32, radius: f32) -> AnyElement {
    match image {
        Some(image) => photo(image, width, height, radius),
        None => missing(width, height, radius),
    }
}

fn photo(image: Arc<RenderImage>, width: f32, height: f32, radius: f32) -> AnyElement {
    div()
        .w(theme::unit(width))
        .h(theme::unit(height))
        .flex_none()
        .rounded(theme::unit(radius))
        .child(
            img(image.clone())
                .w(theme::unit(width))
                .h(theme::unit(height))
                .rounded(theme::unit(radius))
                .object_fit(ObjectFit::Cover),
        )
        .with_animation(
            gpui::ElementId::NamedInteger("cover".into(), image.id.0 as u64),
            Animation::new(Duration::from_millis(120)),
            |cover, delta| cover.opacity(delta),
        )
        .into_any_element()
}

fn missing(width: f32, height: f32, radius: f32) -> AnyElement {
    div()
        .w(theme::unit(width))
        .h(theme::unit(height))
        .flex_none()
        .rounded(theme::unit(radius))
        .bg(theme::ZINC_900)
        .child(
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(icons::icon(
                    Icon::Music,
                    (width * 0.5).clamp(12.0, 64.0),
                    theme::ZINC_600,
                )),
        )
        .into_any_element()
}

/// The box a cover leaves while it is offscreen: the artwork's size and
/// corners without asking for the image or starting a shimmer. The tile keeps
/// its layout, and the frame clock stops paying for rows nobody can see.
pub fn placeholder(width: f32, height: f32, radius: f32) -> AnyElement {
    div()
        .w(theme::unit(width))
        .h(theme::unit(height))
        .flex_none()
        .rounded(theme::unit(radius))
        .bg(theme::ZINC_900)
        .into_any_element()
}

/// The scrim a list puts over the artwork of the song it is playing: a
/// spinner while the stream buffers, then the play/pause state.
pub fn current(
    loading: bool,
    playing: bool,
    radius: f32,
    size: f32,
    accent: Rgba,
) -> impl IntoElement {
    div()
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .rounded(theme::unit(radius))
        .bg(theme::black(0.6))
        .child(if loading {
            icons::spinning(icons::LOADER, size, accent).into_any_element()
        } else {
            icons::icon(
                if playing { icons::MUSIC } else { icons::PAUSE },
                size,
                accent,
            )
            .into_any_element()
        })
}

/// The accent outline Jota draws around the artwork of the song being played.
/// Tailwind's `outline`/`ring` sit outside the image, so the tile keeps every
/// pixel: the band grows out by `width` instead of painting over the photo.
pub fn outline(width: f32, radius: f32, color: Rgba) -> Div {
    div()
        .absolute()
        .top(theme::unit(-width))
        .left(theme::unit(-width))
        .right(theme::unit(-width))
        .bottom(theme::unit(-width))
        .rounded(theme::unit(radius + width))
        .border(theme::unit(width))
        .border_color(color)
}

/// The artwork a playlist shows: its own cover when it has one, the song
/// mosaic only when it does not, and the first song when there is nothing to
/// fill a mosaic with.
pub fn playlist(
    covers: &mut Covers,
    cover: Option<&str>,
    urls: &[String],
    width: f32,
    height: f32,
    radius: f32,
) -> AnyElement {
    if let Some(cover) = cover {
        return art(covers, cover, width, height, radius);
    }

    if urls.len() > 1 {
        return mosaic(covers, urls, width, height, radius);
    }

    art(
        covers,
        urls.first().map_or("", String::as_str),
        width,
        height,
        radius,
    )
}

/// Two covers side by side, or four in a square, with square outer corners.
/// Each cell keeps the shimmer of a single cover until its own image lands.
pub fn mosaic(
    covers: &mut Covers,
    urls: &[String],
    width: f32,
    height: f32,
    radius: f32,
) -> AnyElement {
    if urls.is_empty() {
        return missing(width, height, radius);
    }

    if urls.len() == 1 {
        return art(covers, &urls[0], width, height, radius);
    }

    let four = urls.len() > 2;
    let half = width * 0.5;
    let mut cells = Vec::with_capacity(4);
    for index in 0..4 {
        let cell = if four {
            match urls.get(index) {
                Some(url) => art(covers, url, half, half, 0.0),
                None => missing(half, half, 0.0),
            }
        } else if index == 0 {
            art(covers, &urls[0], half, height, 0.0)
        } else if index == 1 {
            art(covers, &urls[1], half, height, 0.0)
        } else {
            continue;
        };

        cells.push(cell);
    }

    let mut mosaic = div()
        .w(theme::unit(width))
        .h(theme::unit(height))
        .flex_none()
        .rounded(theme::unit(radius))
        .overflow_hidden();
    if four {
        mosaic = mosaic.grid().grid_cols(2);
    } else {
        mosaic = mosaic.flex();
    }

    mosaic.children(cells).into_any_element()
}

/// Jota's `.shimmer`: a #18181b tile with a #1f1f24 band sweeping across it
/// every 1.4s, not a tile that fades in place. The band is painted as clipped
/// slices so a round avatar never shows the square corners of an overflowing
/// gradient.
pub fn shimmer(width: f32, height: f32, radius: f32) -> impl IntoElement {
    let band = (width * 0.6).max(24.0);
    let glow = theme::rgb(0x1f1f24);
    let slices = (width / 3.0).clamp(24.0, 64.0) as usize;
    let step = width / slices as f32;
    div()
        .relative()
        .w(theme::unit(width))
        .h(theme::unit(height))
        .flex_none()
        .rounded(theme::unit(radius))
        .overflow_hidden()
        .bg(theme::ZINC_900)
        .with_animation(
            "cover-shimmer",
            Animation::new(Duration::from_millis(1400))
                .repeat()
                .with_max_fps(60.0),
            move |this, delta| {
                let center = -band + (width + band) * delta;
                let mut cells = Vec::with_capacity(slices);
                for index in 0..slices {
                    let x = index as f32 * step;
                    let distance = ((x + step * 0.5 - center).abs() / (band * 0.5)).min(1.0);
                    let strength = 1.0 - distance;
                    if strength <= 0.0 {
                        continue;
                    }

                    let inset = rounded_inset(width, height, radius, x).max(rounded_inset(
                        width,
                        height,
                        radius,
                        x + step,
                    ));
                    let cell = height - inset * 2.0;
                    if cell <= 0.0 {
                        continue;
                    }

                    cells.push(
                        div()
                            .absolute()
                            .left(theme::unit(x))
                            .top(theme::unit(inset))
                            .w(theme::unit(step + 0.5))
                            .h(theme::unit(cell))
                            .bg(theme::mix(theme::ZINC_900, glow, strength * 0.9)),
                    );
                }

                this.children(cells)
            },
        )
}

fn rounded_inset(width: f32, height: f32, radius: f32, x: f32) -> f32 {
    let radius = radius.min(width.min(height) * 0.5).max(0.0);
    if radius <= 0.0 {
        return 0.0;
    }

    let x = x.clamp(0.0, width);
    let edge = if x < radius {
        radius - x
    } else if x > width - radius {
        x - (width - radius)
    } else {
        return 0.0;
    };

    radius - (radius * radius - edge * edge).max(0.0).sqrt()
}

pub fn brand(source: Source, size: f32) -> AnyElement {
    let (icon, color) = match source {
        Source::Spotify => (Icon::Spotify, theme::SPOTIFY),
        Source::YouTube => (Icon::YouTube, theme::YOUTUBE),
        Source::Local => (Icon::Library, theme::MUTED),
    };

    icons::icon(icon, size, color).into_any_element()
}

pub fn brand_mono(source: Source, size: f32, color: Rgba) -> AnyElement {
    let icon = match source {
        Source::Spotify => Icon::Spotify,
        Source::YouTube => Icon::YouTube,
        Source::Local => Icon::Library,
    };

    icons::icon(icon, size, color).into_any_element()
}

pub fn badge(source: Source, size: f32) -> Div {
    div()
        .w(theme::unit(size))
        .h(theme::unit(size))
        .flex()
        .items_center()
        .justify_center()
        .rounded(theme::unit(6.0))
        .bg(theme::black(0.7))
        .child(brand_mono(source, size - 6.0, theme::MUTED))
}

#[cfg(test)]
#[path = "cover.test.rs"]
mod tests;
