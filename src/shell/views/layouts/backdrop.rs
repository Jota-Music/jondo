//! The ambient backdrop: two soft halos in the dominant colour of the playing
//! cover, and the dust that rises through them.

use std::cell::RefCell;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use gpui::{
    Animation, AnimationExt, Div, ElementId, InteractiveElement, IntoElement, ObjectFit,
    ParentElement, RenderImage, Rgba, Styled, StyledImage, div, img, px, relative,
};

use crate::shared::ui::views::design::theme;

/// The dust, as the web shell draws it: one mote per column with a jitter,
/// because independent draws clump and the eye reads the bare patches first.
/// They spawn low because the field rises, and one drawn near the top leaves
/// after a few pixels and sits out most of its cycle.
const DUST: usize = 64;

/// The halo's colour stops here, as a fraction of the texture radius. The rest
/// stays clear, so the glow has no edge where its box ends.
const GLOW_STOP: f32 = 0.6;
const GLOW_OPACITY: f32 = 0.12;
const GLOW_SIZE: u32 = 192;

/// One clock for the whole field: the frame is rebuilt once and every mote
/// reads its own phase off it. The speeds divide it, so a mote that ends a
/// cycle as the clock wraps continues where it left instead of jumping.
const CLOCK: f32 = 360.0;
const SPEEDS: [f32; 4] = [18.0, 20.0, 24.0, 30.0];

/// How far the field rises over one cycle, in viewport heights.
const RISE: f32 = 0.6;

struct Mote {
    left: f32,
    top: f32,
    size: f32,
    peak: f32,
    duration: f32,
    offset: f32,
    sway: bool,
}

thread_local! {
    static GLOW: RefCell<Option<(u32, Arc<RenderImage>)>> = const { RefCell::new(None) };
}

fn dust_field() -> &'static [Mote] {
    static FIELD: OnceLock<Vec<Mote>> = OnceLock::new();
    FIELD.get_or_init(|| {
        (0..DUST)
            .map(|id| Mote {
                left: (id as f32 + rand::random::<f32>()) / DUST as f32,
                top: 0.55 + rand::random::<f32>() * 0.45,
                size: 2.0 + rand::random::<f32>() * 2.5,
                peak: 0.15 + rand::random::<f32>() * 0.3,
                duration: SPEEDS[id % SPEEDS.len()],
                offset: rand::random::<f32>(),
                sway: rand::random::<bool>(),
            })
            .collect()
    })
}

/// The whole backdrop. `dominant` is the cover's dominant colour, the same one
/// the chrome paints its controls with and the web shell calls
/// `--dominant-color`; `height` is the viewport, which is what it measures its
/// gradient and its rise against.
pub fn show(dominant: Rgba, width: f32, height: f32, motes: bool) -> Div {
    let field = div()
        .absolute()
        .inset_0()
        .overflow_hidden()
        .debug_selector(|| "ambient-backdrop".to_owned())
        .child(halos(dominant, width, height));

    if motes {
        field.child(dust(dominant, height * RISE))
    } else {
        field
    }
}

/// Two halos, one at 30% of the width over the top edge and one at 85% under
/// the bottom edge, each reaching 60% of the way to the farthest corner. GPUI
/// has no radial gradient, so the falloff is painted into a texture once per
/// accent and sampled as a sprite: one quad per halo, and no seam where the
/// box ends.
fn halos(dominant: Rgba, width: f32, height: f32) -> Div {
    let glow = glow(dominant);
    // The farthest corner from each centre: 70% of the width from the top one,
    // 85% from the bottom one, both at the far edge of the height.
    let top = reach(width, height, 0.7);
    let bottom = reach(width, height, 0.85);

    div()
        .absolute()
        .inset_0()
        .child(
            div()
                .absolute()
                .left(px(0.3 * width - top * 0.5))
                .top(px(-top * 0.5))
                .w(px(top))
                .h(px(top))
                .child(img(glow.clone()).size_full().object_fit(ObjectFit::Fill)),
        )
        .child(
            div()
                .absolute()
                .left(px(0.85 * width - bottom * 0.5))
                .bottom(px(-bottom * 0.5))
                .w(px(bottom))
                .h(px(bottom))
                .child(img(glow).size_full().object_fit(ObjectFit::Fill)),
        )
}

/// The side of the square a halo is painted in. The web gradient is a circle
/// from its centre to the farthest corner, drawn to 60%; the texture carries
/// the same 60% in its own radius, so the square is just twice that corner.
fn reach(width: f32, height: f32, at: f32) -> f32 {
    2.0 * ((at * width).powi(2) + height.powi(2)).sqrt()
}

fn dust(dominant: Rgba, rise: f32) -> impl IntoElement {
    div()
        .absolute()
        .inset_0()
        .overflow_hidden()
        .debug_selector(|| "ambient-dust".to_owned())
        .with_animation(
            ElementId::Name("motes".into()),
            Animation::new(Duration::from_secs_f32(CLOCK))
                .repeat()
                .with_max_fps(30.0),
            move |field, delta| {
                let elapsed = delta * CLOCK;
                let mut field = field;

                for mote in dust_field() {
                    let phase = (elapsed / mote.duration + mote.offset).fract();
                    // The ends fade and the middle holds one opacity, so the field
                    // reads as depth instead of one flat sheet.
                    let fade = if phase < 0.08 {
                        phase / 0.08
                    } else if phase < 0.88 {
                        1.0
                    } else {
                        1.0 - (phase - 0.88) / 0.12
                    };
                    let drift = if !mote.sway {
                        0.0
                    } else if phase < 0.5 {
                        -32.0 * phase
                    } else {
                        -16.0 + 72.0 * (phase - 0.5)
                    };

                    field = field.child(
                        div()
                            .absolute()
                            .left(relative(mote.left))
                            .top(relative(mote.top))
                            .child(
                                div()
                                    .relative()
                                    .left(theme::unit(drift))
                                    .top(px(-rise * phase))
                                    .w(theme::unit(mote.size))
                                    .h(theme::unit(mote.size))
                                    .rounded_full()
                                    .bg(dominant)
                                    .opacity(mote.peak * fade),
                            ),
                    );
                }

                field
            },
        )
}

/// One texture at a time: the colour follows the cover, and a cache keyed by it
/// keeps a frame from re-baking the falloff.
fn glow(dominant: Rgba) -> Arc<RenderImage> {
    let key: u32 = dominant.into();
    GLOW.with(|cell| {
        let mut cell = cell.borrow_mut();
        if let Some((cached, image)) = cell.as_ref()
            && *cached == key
        {
            return image.clone();
        }

        let image = Arc::new(RenderImage::new(vec![image::Frame::new(glow_pixels(
            dominant,
        ))]));
        *cell = Some((key, image.clone()));
        image
    })
}

fn glow_pixels(dominant: Rgba) -> image::RgbaImage {
    let size = GLOW_SIZE;
    let radius = size as f32 * 0.5;
    let limit = radius * GLOW_STOP;
    // Straight alpha: the sprite shader premultiplies the sampled colour, so
    // baking the falloff into the channels as well would darken it twice.
    let red = (dominant.r * 255.0).round() as u8;
    let green = (dominant.g * 255.0).round() as u8;
    let blue = (dominant.b * 255.0).round() as u8;
    let mut out = image::RgbaImage::new(size, size);

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 + 0.5 - radius;
            let dy = y as f32 + 0.5 - radius;
            let falloff = (1.0 - (dx * dx + dy * dy).sqrt() / limit).clamp(0.0, 1.0);
            // The atlas uploads BGRA, like the covers do, so the channels go
            // in that order: red and blue the other way round paint the halo
            // in a colour the cover never had.
            out.put_pixel(
                x,
                y,
                image::Rgba([
                    blue,
                    green,
                    red,
                    (GLOW_OPACITY * falloff * 255.0).round() as u8,
                ]),
            );
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::{GLOW_OPACITY, GLOW_STOP, glow, glow_pixels};
    use crate::shared::ui::views::design::theme;
    use std::sync::Arc;

    /// The halo is the dominant colour, not a fixed tint: a new cover rebakes
    /// it, and the same cover keeps the texture it already has.
    #[test]
    fn the_glow_follows_the_dominant_colour() {
        let pink = glow(theme::ACCENT);
        assert!(
            Arc::ptr_eq(&pink, &glow(theme::ACCENT)),
            "the same colour must reuse its texture"
        );

        let green = glow(theme::rgb(0x1db954));
        assert!(
            !Arc::ptr_eq(&pink, &green),
            "another dominant colour must rebake the halo"
        );
    }

    /// The halo is pixels, not a gradient the GPU draws: its centre is the
    /// accent at the shell's opacity and it reaches nothing well before the
    /// texture edge, so the box it is painted in never shows a seam.
    #[test]
    fn the_glow_fades_to_nothing_inside_its_own_texture() {
        let image = glow_pixels(theme::ACCENT);
        let size = image.width();
        let centre = image.get_pixel(size / 2, size / 2);
        let peak = (GLOW_OPACITY * 255.0).round() as i32;
        assert!(
            (i32::from(centre[3]) - peak).abs() <= 2,
            "the centre should carry the shell's opacity, not {}",
            centre[3]
        );
        assert!(centre[0] > 0 && centre[1] > 0 && centre[2] > 0);

        // BGRA, as the atlas expects: the accent's channels land in their own
        // slot and the halo keeps the colour the cover gave it.
        assert_eq!(
            [centre[0], centre[1], centre[2]],
            [0xd6, 0x6d, 0xff],
            "the texture must carry the dominant colour in atlas order"
        );

        let past = (size as f32 * 0.5 * (1.0 + GLOW_STOP) + 1.0) as u32;
        assert!(past < size, "the stop has to land inside the texture");
        assert_eq!(image.get_pixel(past, size / 2)[3], 0);
        assert_eq!(image.get_pixel(0, 0)[3], 0);
    }
}
