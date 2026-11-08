use gpui::{FontWeight, Pixels, Rgba, px, transparent_black};
use gpui_kit::component::{Theme, ThemeMode};

pub const BG: Rgba = rgb(0x0c0a09);
pub const SURFACE: Rgba = rgb(0x09090b);
pub const BORDER: Rgba = rgb(0x27272a);
pub const ZINC_100: Rgba = rgb(0xf4f4f5);
pub const ZINC_200: Rgba = rgb(0xe4e4e7);
pub const ZINC_300: Rgba = rgb(0xd4d4d8);
pub const NEUTRAL_300: Rgba = rgb(0xd4d4d4);
pub const ZINC_900: Rgba = rgb(0x18181b);
pub const ZINC_800: Rgba = rgb(0x27272a);
pub const NEUTRAL_800: Rgba = rgb(0x262626);
pub const ZINC_700: Rgba = rgb(0x3f3f46);
pub const ZINC_600: Rgba = rgb(0x52525b);
pub const SCROLL_THUMB: Rgba = rgb(0x1b1b1f);
pub const TEXT: Rgba = rgb(0xfafafa);
pub const MUTED: Rgba = rgb(0xa1a1aa);
pub const FAINT: Rgba = rgb(0x71717a);
pub const DANGER: Rgba = rgb(0xf87171);
pub const GREEN: Rgba = rgb(0x4ade80);
pub const AMBER: Rgba = rgb(0xfacc15);
pub const AMBER_200: Rgba = rgb(0xfde68a);
pub const AMBER_300: Rgba = rgb(0xfcd34d);
pub const AMBER_500: Rgba = rgb(0xf59e0b);
pub const SPOTIFY: Rgba = rgb(0x1db954);
pub const YOUTUBE: Rgba = rgb(0xff0033);

pub const ACCENT: Rgba = rgb(0xff6dd6);
pub const BINARY: Rgba = rgb(0x000000);

pub const CONTENT_WIDTH: f32 = 672.0;
pub const HEADER_HEIGHT: f32 = 40.0;
/// Height of the phone-only bottom navigation, reserved before it is drawn.
pub const NAV_HEIGHT: f32 = 56.0;
/// Height of the compact player bar: an 8px seek line and a 12px padded row
/// around the 48px cover.
pub const PLAYER_BAR: f32 = 80.0;
pub const PLAYER_DISC: f32 = 192.0;

pub const FAMILY: &str = "Inter Variable";
/// Android has no DejaVu; its mono is Droid Sans Mono. A missing family is a
/// panic inside GPUI's text system, not a fallback.
#[cfg(target_os = "android")]
pub const MONO: &str = "Droid Sans Mono";
#[cfg(not(target_os = "android"))]
pub const MONO: &str = "DejaVu Sans Mono";

/// The base font the design scale is drawn against. The theme installs it as
/// the window rem, so every design unit under `follow` scales with it.
const BASE_FONT: f32 = 14.0;

thread_local! {
    static SCALE: std::cell::Cell<f32> = const { std::cell::Cell::new(1.0) };
}

/// Keeps the design scale in step with the window's base font. Root applies
/// `theme.font_size` to the window rem before the tree is measured.
pub fn follow(window: &gpui::Window) {
    let scale = window.rem_size().as_f32() / BASE_FONT;
    SCALE.with(|value| value.set(scale.clamp(0.5, 4.0)));
}

/// A design pixel: the value the layout was drawn with at the base font,
/// scaled by the window's current rem size.
pub fn unit(value: f32) -> Pixels {
    px(value * SCALE.with(|scale| scale.get()))
}

pub const fn rgb(hex: u32) -> Rgba {
    Rgba {
        r: ((hex >> 16) & 0xff) as f32 / 255.0,
        g: ((hex >> 8) & 0xff) as f32 / 255.0,
        b: (hex & 0xff) as f32 / 255.0,
        a: 1.0,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Weight {
    Regular,
    Medium,
    SemiBold,
}

impl Weight {
    pub fn value(self) -> FontWeight {
        match self {
            Weight::Regular => FontWeight::NORMAL,
            Weight::Medium => FontWeight::MEDIUM,
            Weight::SemiBold => FontWeight::SEMIBOLD,
        }
    }
}

pub fn alpha(color: Rgba, alpha: f32) -> Rgba {
    Rgba {
        a: color.a * alpha.clamp(0.0, 1.0),
        ..color
    }
}

pub fn white(alpha: f32) -> Rgba {
    Rgba {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: alpha,
    }
}

pub fn black(alpha: f32) -> Rgba {
    Rgba {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: alpha,
    }
}

pub fn mix(from: Rgba, to: Rgba, t: f32) -> Rgba {
    let t = t.clamp(0.0, 1.0);
    Rgba {
        r: from.r + (to.r - from.r) * t,
        g: from.g + (to.g - from.g) * t,
        b: from.b + (to.b - from.b) * t,
        a: from.a + (to.a - from.a) * t,
    }
}

pub fn shade(color: Rgba, factor: f32) -> Rgba {
    Rgba {
        r: (color.r * factor).min(1.0),
        g: (color.g * factor).min(1.0),
        b: (color.b * factor).min(1.0),
        a: color.a,
    }
}

pub fn install(cx: &mut gpui::App) {
    cx.text_system()
        .add_fonts(vec![
            std::borrow::Cow::Borrowed(
                include_bytes!("../../../../../assets/fonts/Inter-Regular.ttf").as_slice(),
            ),
            std::borrow::Cow::Borrowed(
                include_bytes!("../../../../../assets/fonts/Inter-Medium.ttf").as_slice(),
            ),
            std::borrow::Cow::Borrowed(
                include_bytes!("../../../../../assets/fonts/Inter-SemiBold.ttf").as_slice(),
            ),
        ])
        .ok();

    Theme::change(ThemeMode::Dark, None, cx);

    Theme::update(cx, |theme| {
        theme.font_family = FAMILY.into();
        theme.mono_font_family = MONO.into();
        theme.font_size = px(14.0);
        theme.radius = px(6.0);
        theme.radius_lg = px(12.0);
        theme.colors.background = BG.into();
        theme.colors.foreground = TEXT.into();
        theme.colors.border = BORDER.into();
        theme.colors.primary = ACCENT.into();
        theme.colors.primary_foreground = BINARY.into();
        theme.colors.primary_hover = shade(ACCENT, 1.08).into();
        theme.colors.primary_active = shade(ACCENT, 0.92).into();
        theme.colors.accent = ZINC_800.into();
        theme.colors.accent_foreground = TEXT.into();
        theme.colors.caret = TEXT.into();
        theme.colors.input = SURFACE.into();
        theme.colors.muted = ZINC_900.into();
        theme.colors.muted_foreground = MUTED.into();
        theme.colors.popover = SURFACE.into();
        theme.colors.popover_foreground = TEXT.into();
        theme.colors.ring = ACCENT.into();
        theme.colors.selection = alpha(ACCENT, 0.28).into();
        theme.colors.slider_bar = ZINC_700.into();
        theme.colors.slider_thumb = ACCENT.into();
        theme.colors.switch = ZINC_700.into();
        theme.colors.scrollbar = transparent_black();
        theme.colors.scrollbar_thumb = SCROLL_THUMB.into();
        theme.colors.scrollbar_thumb_hover = ZINC_600.into();
        theme.colors.danger = DANGER.into();
        theme.colors.danger_foreground = BINARY.into();
        theme.colors.warning = AMBER.into();
        theme.colors.success = GREEN.into();
        theme.colors.overlay = black(0.55).into();
        theme.colors.title_bar = SURFACE.into();
        theme.colors.title_bar_border = BORDER.into();
    });
}
