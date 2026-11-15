//! The ambient backdrop: two soft halos in the dominant colour of the playing
//! cover, and the dust that rises through them.

use std::sync::OnceLock;
use std::time::Duration;

use gpui::{
    Animation, AnimationExt, Div, ElementId, InteractiveElement, IntoElement, ParentElement, Rgba,
    Styled, div, px, relative,
};

use crate::shared::ui::views::components::glow;
use crate::shared::ui::views::design::theme;

/// The dust, as the web shell draws it: one mote per column with a jitter,
/// because independent draws clump and the eye reads the bare patches first.
/// They spawn low because the field rises, and one drawn near the top leaves
/// after a few pixels and sits out most of its cycle.
const DUST: usize = 64;

/// The halo's colour stops here, as a fraction of the texture radius. The rest
/// stays clear, so the glow has no edge where its box ends.
const GLOW_STOP: f32 = 0.6;

/// The dust's radius, as the field paints it. The knob's 100% is 1.25x that:
/// the size the glow reads best at, not the one the field starts from.
const MOTE_SCALE: f32 = 1.25;

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

/// The whole backdrop, in the terms the shell has to hand.
pub struct Backdrop {
    /// The cover's dominant colour, the same one the chrome paints its controls
    /// with and the web shell calls `--dominant-color`.
    pub dominant: Rgba,
    /// The viewport, which is what the gradient and the rise measure against.
    pub width: f32,
    pub height: f32,
    pub motes: bool,
    /// The glow's opacity, from the settings field.
    pub strength: f32,
    /// The dust's size, as the knob leaves it: the field scales it again.
    pub mote: f32,
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

pub fn show(backdrop: Backdrop) -> Div {
    let Backdrop {
        dominant,
        width,
        height,
        motes,
        strength,
        mote,
    } = backdrop;
    let field = div()
        .absolute()
        .inset_0()
        .overflow_hidden()
        .debug_selector(|| "ambient-backdrop".to_owned())
        .child(halos(dominant, strength, width, height));

    if motes {
        field.child(dust(dominant, mote * MOTE_SCALE, height * RISE))
    } else {
        field
    }
}

/// Two halos, one at 30% of the width over the top edge and one at 85% under
/// the bottom edge, each reaching 60% of the way to the farthest corner.
fn halos(dominant: Rgba, strength: f32, width: f32, height: f32) -> Div {
    let texture = glow::texture(dominant, strength, GLOW_STOP);
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
                .child(glow::show_texture(texture.clone(), px(top))),
        )
        .child(
            div()
                .absolute()
                .left(px(0.85 * width - bottom * 0.5))
                .bottom(px(-bottom * 0.5))
                .child(glow::show_texture(texture, px(bottom))),
        )
}

/// The side of the square a halo is painted in. The web gradient is a circle
/// from its centre to the farthest corner, drawn to 60%; the texture carries
/// the same 60% in its own radius, so the square is just twice that corner.
fn reach(width: f32, height: f32, at: f32) -> f32 {
    2.0 * ((at * width).powi(2) + height.powi(2)).sqrt()
}

fn dust(dominant: Rgba, scale: f32, rise: f32) -> impl IntoElement {
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
                                    .w(theme::unit(mote.size * scale))
                                    .h(theme::unit(mote.size * scale))
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
