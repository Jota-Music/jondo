use std::fs;
use std::path::PathBuf;

use sha2::{Digest, Sha256};

use super::cache::*;
/// Pixels each tier is decoded at. A row asks for the small one; the vinyl and
/// the page headers ask for the large one.
use super::*;

/// Whether an entry can leave the cache. An image the previous frame painted
/// gets one frame of grace, so a load arriving after it was drawn cannot evict
/// it before the next tree asks for it again.
pub(super) fn evictable(last: u64, frame: u64) -> bool {
    last + 1 < frame
}

pub(super) fn load(client: &reqwest::blocking::Client, url: &str, tier: usize) -> Fetched {
    let source = read_disk(url).or_else(|| download(client, url));
    let Some(source) = source else {
        return Fetched {
            url: url.to_owned(),
            tier,
            image: None,
            color: None,
        };
    };

    let color = crate::shared::ui::views::design::color::dominant(&source);
    let size = SIZES[tier];
    let image = if source.dimensions() == (size, size) {
        source
    } else {
        image::imageops::resize(&source, size, size, image::imageops::FilterType::Triangle)
    };

    Fetched {
        url: url.to_owned(),
        tier,
        image: Some(to_bgra(image)),
        color: Some(color),
    }
}

/// The atlas uploads BGRA. Doing it here keeps the per-pixel walk on the worker
/// thread: the UI thread only wraps the bytes and hands them to the renderer,
/// so a burst of covers landing mid-resize cannot stall a frame.
pub(super) fn to_bgra(image: image::RgbaImage) -> image::RgbaImage {
    let (width, height) = image.dimensions();
    let mut bytes = image.into_raw();
    for pixel in bytes.as_chunks_mut::<4>().0 {
        pixel.swap(0, 2);
    }

    image::RgbaImage::from_raw(width, height, bytes).expect("cover keeps its dimensions")
}

pub(super) fn download(client: &reqwest::blocking::Client, url: &str) -> Option<image::RgbaImage> {
    let response = client
        .get(url)
        .header(reqwest::header::USER_AGENT, "Mozilla/5.0 Jondo")
        .send();

    let bytes = match response.and_then(|response| response.bytes()) {
        Ok(bytes) => bytes,
        Err(error) => {
            log::debug!("cover failed {url}: {error}");
            return None;
        }
    };

    let decoded = match image::load_from_memory(&bytes) {
        Ok(image) => image,
        Err(error) => {
            log::debug!("cover decode failed {url}: {error}");
            return None;
        }
    };

    let thumb = decoded
        .resize_to_fill(
            COVER_SIZE,
            COVER_SIZE,
            image::imageops::FilterType::Triangle,
        )
        .to_rgba8();
    write_disk(url, &thumb);
    Some(thumb)
}

pub(super) fn cache_dir() -> Option<PathBuf> {
    Some(crate::shared::storage::dir()?.join("covers"))
}

/// Where a cover lives on disk once fetched. The media session reads it to
/// publish the artwork, so it must not fetch a second time.
pub fn path(url: &str) -> Option<PathBuf> {
    cache_path(url)
}

pub(super) fn cache_path(url: &str) -> Option<PathBuf> {
    Some(cache_dir()?.join(format!("{:x}.jpg", Sha256::digest(url.as_bytes()))))
}

pub(super) fn read_disk(url: &str) -> Option<image::RgbaImage> {
    let bytes = fs::read(cache_path(url)?).ok()?;
    Some(image::load_from_memory(&bytes).ok()?.to_rgba8())
}

pub(super) fn write_disk(url: &str, thumb: &image::RgbaImage) {
    use image::ImageEncoder as _;

    let Some(path) = cache_path(url) else {
        return;
    };

    if let Some(dir) = path.parent()
        && let Err(error) = fs::create_dir_all(dir)
    {
        log::warn!("cannot create {}: {error}", dir.display());
        return;
    }

    let rgb = image::DynamicImage::ImageRgba8(thumb.clone()).into_rgb8();
    let mut buffer = Vec::new();
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buffer, 80);
    if let Err(error) = encoder.write_image(
        rgb.as_raw(),
        rgb.width(),
        rgb.height(),
        image::ExtendedColorType::Rgb8,
    ) {
        log::warn!("cannot encode cover {url}: {error}");
        return;
    }

    if let Err(error) = fs::write(&path, buffer) {
        log::warn!("cannot save {}: {error}", path.display());
    }
}
