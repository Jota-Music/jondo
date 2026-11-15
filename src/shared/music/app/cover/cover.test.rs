/// Pixels each tier is decoded at. A row asks for the small one; the vinyl and
/// the page headers ask for the large one.
use super::*;
use super::{cache::*, load::*, spin::*};

fn marked() -> image::RgbaImage {
    let mut image = image::RgbaImage::new(SPIN_SIZE, SPIN_SIZE);
    image.put_pixel(SPIN_SIZE / 2, 4, image::Rgba([255, 0, 0, 255]));
    image
}

fn red(image: &image::RgbaImage, x: u32, y: u32) -> bool {
    (x.saturating_sub(1)..=x + 1)
        .flat_map(|x| (y.saturating_sub(1)..=y + 1).map(move |y| (x, y)))
        .any(|(x, y)| image.get_pixel(x, y)[0] > 100)
}

#[test]
fn rotation_carries_the_disc_around() {
    let source = marked();
    let middle = SPIN_SIZE / 2;

    assert!(red(&rotate(&source, 0.0), middle, 4));
    assert!(red(
        &rotate(&source, std::f32::consts::FRAC_PI_2),
        SPIN_SIZE - 5,
        middle
    ));
    assert!(red(
        &rotate(&source, std::f32::consts::PI),
        middle,
        SPIN_SIZE - 5
    ));
    assert!(red(
        &rotate(&source, 3.0 * std::f32::consts::FRAC_PI_2),
        4,
        middle
    ));
}

#[test]
fn a_small_slot_shares_the_thumb_and_a_large_one_takes_the_full_size() {
    assert_eq!(tier(80.0), THUMB);
    assert_eq!(tier(160.0), THUMB);
    assert_eq!(tier(280.0), FULL);
    assert_eq!(tier(384.0), FULL);
}

#[test]
fn a_cover_painted_in_the_previous_frame_survives_eviction() {
    assert!(!evictable(4, 5));
    assert!(evictable(3, 5));
}

/// A cover URL is untrusted input, so only the image origins the sources serve
/// are fetched, and only over https.
#[test]
fn only_a_source_image_host_is_fetched() {
    assert!(allowed("https://i.scdn.co/image/abc"));
    assert!(allowed("https://thisis-images.scdn.co/abc"));
    assert!(allowed("https://i.ytimg.com/vi/abc/mqdefault.jpg"));
    assert!(allowed("https://lh3.googleusercontent.com/abc"));

    assert!(!allowed("http://i.scdn.co/image/abc"), "https only");
    assert!(!allowed("https://localhost:8080/cover.jpg"));
    assert!(!allowed("https://192.168.1.10/cover.jpg"));
    assert!(!allowed("https://i.scdn.co.evil.example/cover.jpg"));
    assert!(!allowed("file:///etc/passwd"));
    assert!(!allowed(""));
}

/// A file the screen could never show is refused before its pixels are
/// decoded: the size in the header is what decides.
#[test]
fn a_cover_that_would_blow_up_memory_is_refused() {
    assert!(fits(COVER_SIZE, COVER_SIZE));
    assert!(fits(32, 32));
    assert!(!fits(0, 320));
    assert!(!fits(320, 0));
    assert!(!fits(20_000, 20_000), "400M pixels is not a cover");
    assert!(!fits(10_000, 100), "one row of a banner is not a cover");
}

#[test]
fn a_cover_reaches_the_atlas_in_bgra() {
    let mut image = image::RgbaImage::new(1, 1);
    image.put_pixel(0, 0, image::Rgba([1, 2, 3, 4]));
    assert_eq!(to_bgra(image).get_pixel(0, 0), &image::Rgba([3, 2, 1, 4]));
}
