use super::*;

#[test]
fn an_appimage_is_picked_for_this_arch() {
    let release = serde_json::json!({
        "assets": [
            {"name": "SHA256SUMS", "browser_download_url": "https://example/sums"},
            {"name": "jondo-macos.zip", "browser_download_url": "https://example/zip"},
            {"name": format!("jondo-{}.AppImage", std::env::consts::ARCH), "browser_download_url": "https://example/appimage"}
        ]
    });
    assert_eq!(
        asset(&release, Channel::AppImage),
        Some((
            format!("jondo-{}.AppImage", std::env::consts::ARCH),
            "https://example/appimage".to_owned()
        ))
    );
}

#[test]
fn an_apk_is_picked_for_this_arch() {
    let release = serde_json::json!({
        "assets": [
            {"name": "SHA256SUMS", "browser_download_url": "https://example/sums"},
            {"name": "jondo-arm64-v8a.apk", "browser_download_url": "https://example/jondo-arm64-v8a.apk"},
            {"name": "jondo-x86_64.apk", "browser_download_url": "https://example/jondo-x86_64.apk"}
        ]
    });
    let expected = package();
    assert_eq!(
        asset(&release, Channel::Android),
        Some((expected.to_owned(), format!("https://example/{expected}")))
    );
}

/// The package this processor runs, the way the release names it.
fn package() -> &'static str {
    match std::env::consts::ARCH {
        "aarch64" => "jondo-arm64-v8a.apk",
        "x86_64" => "jondo-x86_64.apk",
        other => panic!("no Android package for {other}"),
    }
}

#[test]
fn a_channel_without_self_update_picks_nothing() {
    let release = serde_json::json!({
        "assets": [{"name": "jondo.apk", "browser_download_url": "https://example/apk"}]
    });
    assert_eq!(asset(&release, Channel::External), None);
}

#[test]
fn a_macos_archive_is_picked_for_this_arch() {
    let release = serde_json::json!({
        "assets": [
            {"name": "jondo-macos-aarch64.zip", "browser_download_url": "https://example/jondo-macos-aarch64.zip"},
            {"name": "jondo-macos-x86_64.zip", "browser_download_url": "https://example/jondo-macos-x86_64.zip"}
        ]
    });
    let expected = match std::env::consts::ARCH {
        "aarch64" => "jondo-macos-aarch64.zip",
        "x86_64" => "jondo-macos-x86_64.zip",
        other => panic!("no macOS archive for {other}"),
    };
    assert_eq!(
        asset(&release, Channel::Macos),
        Some((expected.to_owned(), format!("https://example/{expected}")))
    );
}

#[test]
fn another_apps_assets_are_never_picked() {
    let release = serde_json::json!({
        "assets": [
            {"name": "jota.apk", "browser_download_url": "https://example/jota"},
            {"name": "jondo.apk.sha256", "browser_download_url": "https://example/sum"},
            {"name": format!("jota-{}.AppImage", std::env::consts::ARCH), "browser_download_url": "https://example/image"}
        ]
    });
    assert_eq!(asset(&release, Channel::Android), None);
    assert_eq!(asset(&release, Channel::AppImage), None);
}

#[test]
fn the_package_of_another_architecture_is_not_picked() {
    let release = serde_json::json!({
        "assets": [
            {"name": "jondo-arm64-v8a.apk", "browser_download_url": "https://example/arm64"},
            {"name": "jondo-x86_64.apk", "browser_download_url": "https://example/x86"}
        ]
    });
    let (name, _) = asset(&release, Channel::Android).expect("a package for this arch");
    assert_eq!(name, package());
    assert_ne!(name, "jondo.apk");
}

#[test]
fn the_checksum_listing_is_read_by_file_name() {
    let listing = "\
aaaa  *jondo.apk
bbbb  ./dist/jondo-x86_64.AppImage
";
    assert_eq!(parse_sums(listing, "jondo.apk"), Some("aaaa".to_owned()));
    assert_eq!(
        parse_sums(listing, "jondo-x86_64.AppImage"),
        Some("bbbb".to_owned())
    );
    assert_eq!(parse_sums(listing, "missing.apk"), None);
}

#[test]
fn the_digest_matches_the_file_bytes() {
    let path = std::env::temp_dir().join(format!("jondo-digest-{}", std::process::id()));
    std::fs::write(&path, b"NEW APPIMAGE BYTES").unwrap();
    let got = digest(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    assert_eq!(
        got,
        "ebfb6ea664240fa50c62008123a8f204cf3b03ffb06da312d3e24e489cbfab61"
    );
}

#[test]
fn a_newer_version_is_recognised() {
    assert!(newer("0.2.0", "0.1.0"));
    assert!(newer("1.0.0", "0.9.9"));
    assert!(newer("v0.2.0", "0.1.0"));
    assert!(!newer("0.1.0", "0.1.0"));
    assert!(!newer("0.0.9", "0.1.0"));
}

#[test]
fn a_version_that_is_not_a_triple_is_ignored() {
    assert!(!newer("dev", "0.1.0"));
    assert!(!newer("0.2", "0.1.0"));
    assert!(!newer("0.2.0.1", "0.1.0"));
    assert_eq!(parse("v0.2.0-rc1"), Some([0, 2, 0]));
    assert_eq!(parse("0.2.0+build"), Some([0, 2, 0]));
}
