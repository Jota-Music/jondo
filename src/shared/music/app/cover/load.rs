use std::io::Read;

use super::cache::*;
/// Pixels each tier is decoded at. A row asks for the small one; the vinyl and
/// the page headers ask for the large one.
use super::*;

/// A cover file past this size is refused before it is read, so a response the
/// screen could never show does not sit in memory first.
const MAX_SOURCE: u64 = 8 << 20;

/// The largest image worth decoding. A few hundred kilobytes of JPEG can name a
/// 20000×20000 canvas, which decodes into gigabytes: the header is checked
/// before the pixels are.
const MAX_PIXELS: u64 = 40 << 20;

/// How far from square a cover may be. The UI crops every cover to a square, so
/// a banner-shaped image is not art the shelf can show.
const MAX_ASPECT: u32 = 8;

/// The image domains a cover may come from. A cover URL is untrusted: it can
/// arrive in a relay room's song or in a backup another account wrote. Fetching
/// only the sources' own CDNs, over https, keeps the app from being pointed at
/// the user's network. The whole domain is allowed because Spotify and YouTube
/// spread their art across subdomains (`i.`, `mosaic.`, `thisis-images.` …).
const DOMAINS: &[&str] = &[
    "scdn.co",
    "spotifycdn.com",
    "ytimg.com",
    "ggpht.com",
    "googleusercontent.com",
];

/// Whether the URL is an https image on a domain a source actually serves.
pub(super) fn allowed(url: &str) -> bool {
    let Ok(parsed) = url::Url::parse(url) else {
        return false;
    };

    if parsed.scheme() != "https" {
        return false;
    }

    let Some(host) = parsed.host_str() else {
        return false;
    };

    DOMAINS
        .iter()
        .any(|domain| host == *domain || host.ends_with(&format!(".{domain}")))
}

/// Whether an entry can leave the cache. An image the previous frame painted
/// gets one frame of grace, so a load arriving after it was drawn cannot evict
/// it before the next tree asks for it again.
pub(super) fn evictable(last: u64, frame: u64) -> bool {
    last + 1 < frame
}

/// A cover is fetched, decoded and kept in memory: nothing is written to disk,
/// so the folder the app owns does not grow with what the user browses.
pub(super) fn load(client: &reqwest::blocking::Client, url: &str, tier: usize) -> Fetched {
    let Some(source) = download(client, url) else {
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
    if !allowed(url) {
        log::debug!("cover refused {url}");
        return None;
    }

    let response = client
        .get(url)
        .header(reqwest::header::USER_AGENT, "Mozilla/5.0 Jondo")
        .send()
        .ok()?;

    if !response.status().is_success() {
        log::debug!("cover {url} answered {}", response.status());
        return None;
    }

    if response
        .content_length()
        .is_some_and(|length| length > MAX_SOURCE)
    {
        log::debug!("cover {url} is past the size limit");
        return None;
    }

    let mut raw = Vec::new();
    if let Err(error) = response.take(MAX_SOURCE).read_to_end(&mut raw) {
        log::debug!("cover failed {url}: {error}");
        return None;
    }

    // A small file can decode into a huge image, so the size the header carries
    // is judged before the pixels are decoded.
    let (width, height) = dimensions(&raw)?;
    if !fits(width, height) {
        log::debug!("cover {url} is {width}x{height}");
        return None;
    }

    let decoded = match image::load_from_memory(&raw) {
        Ok(image) => image,
        Err(error) => {
            log::debug!("cover decode failed {url}: {error}");
            return None;
        }
    };

    Some(
        decoded
            .resize_to_fill(
                COVER_SIZE,
                COVER_SIZE,
                image::imageops::FilterType::Triangle,
            )
            .to_rgba8(),
    )
}

/// The size the file says it has, read from its header.
fn dimensions(raw: &[u8]) -> Option<(u32, u32)> {
    image::ImageReader::new(std::io::Cursor::new(raw))
        .with_guessed_format()
        .ok()?
        .into_dimensions()
        .ok()
}

/// A cover past the pixel budget, with no pixels, or shaped so far from a square
/// that every row would crop it away, is refused.
pub(super) fn fits(width: u32, height: u32) -> bool {
    if width == 0 || height == 0 {
        return false;
    }

    if u64::from(width) * u64::from(height) > MAX_PIXELS {
        return false;
    }

    width <= MAX_ASPECT * height && height <= MAX_ASPECT * width
}
