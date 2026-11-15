//! A soft radial glow, baked into a texture once and sampled as a sprite.
//!
//! GPUI has no radial gradient, so the falloff is painted into pixels: one quad
//! per glow, and nothing shows where its box ends. A texture fades inside its
//! own square, where a blurred shape paints past it and the ancestor that
//! clips the box cuts the glow.

use std::cell::RefCell;
use std::sync::Arc;

use gpui::{
    Div, ObjectFit, ParentElement, Pixels, RenderImage, Rgba, Styled, StyledImage, div, img,
};

/// The texture is square, painted once and sampled at whatever side the caller
/// draws it in, so its size is the sharpest one a screen asks for.
const SIZE: u32 = 192;

/// How many textures are kept alive: the backdrop's halo follows the cover, and
/// the hint's follows the accent, so a frame asks for a couple of them. Any
/// more and a screen that changes colour every few seconds would grow the
/// cache until the process ends.
const KEPT: usize = 8;

#[derive(Clone, Copy, PartialEq)]
struct Key {
    color: u32,
    alpha: u32,
    stop: u32,
}

thread_local! {
    static CACHE: RefCell<Vec<(Key, Arc<RenderImage>)>> = const { RefCell::new(Vec::new()) };
}

/// The glow as a square of `side`: `color` at `alpha`, reaching nothing by
/// `stop` of its radius.
pub fn show(color: Rgba, alpha: f32, stop: f32, side: Pixels) -> Div {
    show_texture(texture(color, alpha, stop), side)
}

/// The same square for a caller that bakes once and draws it more than once.
pub fn show_texture(texture: Arc<RenderImage>, side: Pixels) -> Div {
    div()
        .flex_none()
        .w(side)
        .h(side)
        .child(img(texture).size_full().object_fit(ObjectFit::Fill))
}

pub fn texture(color: Rgba, alpha: f32, stop: f32) -> Arc<RenderImage> {
    // A tint that already carries an opacity keeps it: the knob adds to what
    // the caller asked for instead of replacing it.
    let alpha = alpha * color.a;
    let key = Key {
        color: color.into(),
        alpha: alpha.to_bits(),
        stop: stop.to_bits(),
    };
    CACHE.with(|cell| {
        let mut cell = cell.borrow_mut();
        if let Some((_, image)) = cell.iter().find(|(cached, _)| *cached == key) {
            return image.clone();
        }

        let image = Arc::new(RenderImage::new(vec![image::Frame::new(pixels(
            color, alpha, stop,
        ))]));
        if cell.len() == KEPT {
            cell.remove(0);
        }
        cell.push((key, image.clone()));
        image
    })
}

fn pixels(color: Rgba, alpha: f32, stop: f32) -> image::RgbaImage {
    let size = SIZE;
    let radius = size as f32 * 0.5;
    let limit = radius * stop;
    // Straight alpha: the sprite shader premultiplies the sampled colour, so
    // baking the falloff into the channels as well would darken it twice.
    let red = (color.r * 255.0).round() as u8;
    let green = (color.g * 255.0).round() as u8;
    let blue = (color.b * 255.0).round() as u8;
    let mut out = image::RgbaImage::new(size, size);

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 + 0.5 - radius;
            let dy = y as f32 + 0.5 - radius;
            let falloff = (1.0 - (dx * dx + dy * dy).sqrt() / limit).clamp(0.0, 1.0);
            // The atlas uploads BGRA, like the covers do, so the channels go
            // in that order: red and blue the other way round paint a glow in
            // a colour its source never had.
            out.put_pixel(
                x,
                y,
                image::Rgba([blue, green, red, (alpha * falloff * 255.0).round() as u8]),
            );
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::{pixels, texture};
    use crate::shared::ui::views::design::theme;
    use std::sync::Arc;

    /// The glow is whatever colour it was asked for: the same request reuses
    /// the texture it already baked, and another colour bakes its own.
    #[test]
    fn the_glow_follows_the_colour_it_is_given() {
        let pink = texture(theme::ACCENT, 0.12, 0.6);
        assert!(
            Arc::ptr_eq(&pink, &texture(theme::ACCENT, 0.12, 0.6)),
            "the same request must reuse its texture"
        );

        let green = texture(theme::rgb(0x1db954), 0.12, 0.6);
        assert!(
            !Arc::ptr_eq(&pink, &green),
            "another colour must bake the glow again"
        );
    }

    /// Every knob is part of the identity: a stronger glow or a wider fade is a
    /// different texture, not the one already in hand.
    #[test]
    fn the_knobs_bake_their_own_texture() {
        let base = texture(theme::ACCENT, 0.12, 0.6);
        assert!(!Arc::ptr_eq(&base, &texture(theme::ACCENT, 0.2, 0.6)));
        assert!(!Arc::ptr_eq(&base, &texture(theme::ACCENT, 0.12, 1.0)));
    }

    /// The glow is pixels, not a shape the GPU draws: its centre carries the
    /// alpha it was asked for and it reaches nothing well before the texture
    /// edge, so the box it is painted in never shows a seam.
    #[test]
    fn the_glow_fades_to_nothing_inside_its_own_texture() {
        let image = pixels(theme::ACCENT, 0.12, 0.6);
        let size = image.width();
        let centre = image.get_pixel(size / 2, size / 2);
        let peak = (0.12f32 * 255.0).round() as i32;
        assert!(
            (i32::from(centre[3]) - peak).abs() <= 2,
            "the centre should carry the alpha it was given, not {}",
            centre[3]
        );
        assert!(centre[0] > 0 && centre[1] > 0 && centre[2] > 0);

        // BGRA, as the atlas expects: the accent's channels land in their own
        // slot and the glow keeps the colour it was given.
        assert_eq!(
            [centre[0], centre[1], centre[2]],
            [0xd6, 0x6d, 0xff],
            "the texture must carry the colour in atlas order"
        );

        let past = (size as f32 * 0.5 * (1.0 + 0.6) + 1.0) as u32;
        assert!(past < size, "the stop has to land inside the texture");
        assert_eq!(image.get_pixel(past, size / 2)[3], 0);
        assert_eq!(image.get_pixel(0, 0)[3], 0);
    }
}
