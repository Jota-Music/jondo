use super::{location::*, widgets::*};

#[test]
fn a_content_uri_name_is_decoded() {
    assert_eq!(
        decoded("primary%3Ajota.backup.json"),
        "primary:jota.backup.json"
    );
    assert_eq!(decoded("jota.backup.json"), "jota.backup.json");
}

#[test]
fn a_place_is_read_from_the_authority() {
    assert_eq!(
        place("content://com.google.android.apps.docs.storage/document/abc"),
        "Google Drive"
    );
    assert_eq!(
        place("content://com.android.externalstorage.documents/tree/primary"),
        "Phone"
    );
    assert_eq!(place("/home/me/backup"), "");
}

#[test]
fn a_location_shows_the_provider_name_or_its_last_segment() {
    assert_eq!(
        shown_name("content://drive/document/abc", "jota.backup.json"),
        "jota.backup.json"
    );
    assert_eq!(
        shown_name("content://drive/document/jota.backup.json", ""),
        "jota.backup.json"
    );
    assert_eq!(shown_name("/home/me/sync", ""), "sync");
}

#[test]
fn a_size_is_read_in_the_unit_that_fits() {
    assert_eq!(size(512), "512 B");
    assert_eq!(size(2048), "2 KB");
    assert_eq!(size(3 * 1024 * 1024), "3.0 MB");
}
