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
fn an_apk_is_picked_for_android() {
    let release = serde_json::json!({
        "assets": [
            {"name": "jondo.apk.sha256", "browser_download_url": "https://example/sum"},
            {"name": "jondo.apk", "browser_download_url": "https://example/apk"}
        ]
    });
    assert_eq!(
        asset(&release, Channel::Android),
        Some(("jondo.apk".to_owned(), "https://example/apk".to_owned()))
    );
}

#[test]
fn a_channel_without_self_update_picks_nothing() {
    let release = serde_json::json!({
        "assets": [{"name": "jondo.apk", "browser_download_url": "https://example/apk"}]
    });
    assert_eq!(asset(&release, Channel::External), None);
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
