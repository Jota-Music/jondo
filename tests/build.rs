//! The build the project ships: the rules that keep it small and the ones
//! that keep it fast, checked instead of remembered.
//!
//! The release profile compiles everything for size and gives the full
//! optimization back to the crates that run again on every frame: layout,
//! text shaping, painting and the audio decoder. A crate that quietly moves
//! from one group to the other changes what the phone does, so the split is
//! written down here.

use std::fs;
use std::path::{Path, PathBuf};

/// The crates the frame path leans on: layout, text shaping, painting and
/// audio. Every frame runs them again, so they keep the full optimization.
const FAST: &[&str] = &[
    "gpui-base",
    "gpui-component",
    "gpui-kit-assets",
    "gpui-pre",
    "gpui-pre-linux",
    "gpui-pre-platform",
    "gpui-pre-wgpu",
    "taffy",
    "cosmic-text",
    "swash",
    "rustybuzz",
    "harfrust",
    "skrifa",
    "wgpu",
    "wgpu-core",
    "wgpu-hal",
    "wgpu-types",
    "naga",
    "lyon",
    "tiny-skia",
    "usvg",
    "resvg",
    "moxcms",
    "image",
    "image-webp",
    "zune-jpeg",
    "png",
    "symphonia-core",
    "symphonia-codec-aac",
    "symphonia-format-isomp4",
    "cpal",
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn manifest() -> String {
    fs::read_to_string(root().join("Cargo.toml")).expect("the manifest is readable")
}

/// The `opt-level` the release profile gives a package, if it names one.
fn opt_level(manifest: &str, package: &str) -> Option<String> {
    let section = format!("[profile.release.package.{package}]");
    let mut lines = manifest.lines().skip_while(|line| line.trim() != section);
    lines.next()?;
    for line in lines {
        let line = line.trim();
        if line.starts_with('[') {
            break;
        }
        if let Some(value) = line.strip_prefix("opt-level") {
            return Some(
                value
                    .trim_start()
                    .trim_start_matches('=')
                    .trim()
                    .trim_matches('"')
                    .to_owned(),
            );
        }
    }
    None
}

/// The release profile's own section, without the packages it names.
fn release_profile(manifest: &str) -> &str {
    let section = manifest
        .split("[profile.release]")
        .nth(1)
        .expect("the release profile is there");
    section
        .split("[profile.release.package.")
        .next()
        .unwrap_or(section)
}

#[test]
fn the_release_profile_compiles_for_size() {
    let manifest = manifest();
    let profile = release_profile(&manifest);
    assert!(
        profile.contains("opt-level = \"z\""),
        "everything that is not on the frame path is compiled for size"
    );
}

#[test]
fn the_frame_path_keeps_its_optimization() {
    let manifest = manifest();
    for package in FAST {
        assert_eq!(
            opt_level(&manifest, package).as_deref(),
            Some("3"),
            "{package} runs on every frame and keeps the full optimization"
        );
    }
}

#[test]
fn the_release_profile_keeps_its_own_settings() {
    let manifest = manifest();
    let profile = release_profile(&manifest);
    assert!(
        profile.contains("panic = \"abort\""),
        "there is no unwinding to keep the tables for"
    );
    assert!(
        profile.contains("strip = true"),
        "the symbols are dead weight in a shipped binary"
    );
    assert!(
        profile.contains("lto = \"fat\""),
        "the linker folds the code that repeats"
    );
}

#[test]
fn the_bundled_emoji_font_stays_small() {
    // The colour bitmap font was ten megabytes and the system already has
    // one on every device that can render it. The outline font covers the
    // same emoji in a twelfth of the bytes.
    let font = root().join("android/app/src/main/assets/fonts/NotoEmoji.ttf");
    let size = fs::metadata(&font).expect("the font is there").len();
    assert!(
        size < 1_000_000,
        "the bundled emoji font is {size} bytes; the colour one was over ten megabytes"
    );
}

#[test]
fn the_android_package_is_not_universal() {
    // A universal package carries the library of every architecture, so it
    // downloads half of them unused and weighs twice as much.
    let gradle =
        fs::read_to_string(root().join("android/app/build.gradle.kts")).expect("the build file");
    assert!(
        gradle.contains("isUniversalApk = false"),
        "the release publishes one package per architecture"
    );
}
