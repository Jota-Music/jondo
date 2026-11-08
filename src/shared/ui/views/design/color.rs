use std::collections::HashMap;

use gpui::Rgba;
use image::{Rgba as Pixel, RgbaImage};

pub fn dominant(image: &RgbaImage) -> Rgba {
    let thumb = image::imageops::resize(image, 100, 100, image::imageops::FilterType::Triangle);
    let mut buckets: HashMap<u32, (u32, u32, u32, u32)> = HashMap::new();
    let mut total = (0u64, 0u64, 0u64, 0u64);

    for Pixel([r, g, b, a]) in thumb.pixels() {
        let (r, g, b, a) = (*r, *g, *b, *a);
        if a < 8 {
            continue;
        }

        total.0 += r as u64;
        total.1 += g as u64;
        total.2 += b as u64;
        total.3 += 1;
        if neutral(r, g, b) {
            continue;
        }

        let key = ((r as u32 >> 4) << 8) | ((g as u32 >> 4) << 4) | (b as u32 >> 4);
        let entry = buckets.entry(key).or_default();
        entry.0 += 1;
        entry.1 += r as u32;
        entry.2 += g as u32;
        entry.3 += b as u32;
    }

    if let Some((count, r, g, b)) = buckets
        .values()
        .max_by_key(|entry| entry.0)
        .filter(|entry| entry.0 > 0)
    {
        let count = *count;
        return normalize((r / count) as u8, (g / count) as u8, (b / count) as u8);
    }

    if total.3 == 0 {
        return crate::shared::ui::views::design::theme::ACCENT;
    }

    normalize(
        (total.0 / total.3) as u8,
        (total.1 / total.3) as u8,
        (total.2 / total.3) as u8,
    )
}

fn neutral(r: u8, g: u8, b: u8) -> bool {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    max - min < 20 || max < 25 || min > 230
}

fn brightness(r: u8, g: u8, b: u8) -> f32 {
    (r as f32 * 299.0 + g as f32 * 587.0 + b as f32 * 114.0) / 1000.0
}

fn normalize(r: u8, g: u8, b: u8) -> Rgba {
    let value = brightness(r, g, b);
    let factor = if value < 80.0 {
        80.0 / value.max(1.0)
    } else if value > 180.0 {
        180.0 / value
    } else {
        1.0
    };

    Rgba {
        r: (r as f32 * factor).clamp(0.0, 255.0) / 255.0,
        g: (g as f32 * factor).clamp(0.0, 255.0) / 255.0,
        b: (b as f32 * factor).clamp(0.0, 255.0) / 255.0,
        a: 1.0,
    }
}

pub fn contrast(color: Rgba) -> Rgba {
    if brightness(
        (color.r * 255.0).round() as u8,
        (color.g * 255.0).round() as u8,
        (color.b * 255.0).round() as u8,
    ) < 140.0
    {
        crate::shared::ui::views::design::theme::rgb(0xffffff)
    } else {
        crate::shared::ui::views::design::theme::rgb(0x000000)
    }
}

pub fn hex(color: Rgba) -> String {
    let channel = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!(
        "#{:02x}{:02x}{:02x}",
        channel(color.r),
        channel(color.g),
        channel(color.b)
    )
}
