use gpui::RenderImage;

/// Pixels each tier is decoded at. A row asks for the small one; the vinyl and
/// the page headers ask for the large one.
use super::*;

pub(super) fn spin_frame(source: &image::RgbaImage, angle: f32) -> RenderImage {
    RenderImage::new(vec![image::Frame::new(rotate(source, angle))])
}

pub(super) fn rotate(source: &image::RgbaImage, angle: f32) -> image::RgbaImage {
    let size = SPIN_SIZE;
    let (width, height) = source.dimensions();
    let (sin, cos) = angle.sin_cos();
    let center_x = width as f32 * 0.5;
    let center_y = height as f32 * 0.5;
    let radius = size as f32 * 0.5;
    let src = source.as_raw();
    let mut out = image::RgbaImage::new(size, size);
    let dst: &mut [u8] = out.as_mut();

    for y in 0..size {
        let dy = y as f32 + 0.5 - radius;
        for x in 0..size {
            let dx = x as f32 + 0.5 - radius;
            let sx = center_x + dx * cos + dy * sin;
            let sy = center_y - dx * sin + dy * cos;
            let pixel = bilinear(src, width, height, sx, sy);
            let at = ((y * size + x) * 4) as usize;
            dst[at..at + 4].copy_from_slice(&pixel);
        }
    }

    out
}

pub(super) fn bilinear(src: &[u8], width: u32, height: u32, x: f32, y: f32) -> [u8; 4] {
    let x = x.clamp(0.5, width as f32 - 0.5) - 0.5;
    let y = y.clamp(0.5, height as f32 - 0.5) - 0.5;
    let x0 = x as usize;
    let y0 = y as usize;
    let x1 = (x0 + 1).min(width as usize - 1);
    let y1 = (y0 + 1).min(height as usize - 1);
    let fx = x - x0 as f32;
    let fy = y - y0 as f32;
    let row0 = y0 * width as usize;
    let row1 = y1 * width as usize;
    let at = |row: usize, column: usize| -> &[u8] {
        let start = (row + column) * 4;
        &src[start..start + 4]
    };

    let mut out = [0u8; 4];
    for (channel, slot) in out.iter_mut().enumerate() {
        let top = at(row0, x0)[channel] as f32 * (1.0 - fx) + at(row0, x1)[channel] as f32 * fx;
        let bottom = at(row1, x0)[channel] as f32 * (1.0 - fx) + at(row1, x1)[channel] as f32 * fx;
        *slot = (top * (1.0 - fy) + bottom * fy).round() as u8;
    }

    out
}
