use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::Duration;

use librespot_protocol::playlist4_external::SelectedListContent;

use crate::shared::music::domain::Song;
use crate::shared::storage::SpotifyAuth;

#[cfg(test)]
use super::*;
use super::{library::*, login::*, playlist::*, radio::*, session::*, tracks::*};

#[test]
fn an_id_shaped_query_is_opened_as_that_track() {
    assert_eq!(
        track_id("4uLU6hMCjMI75M1A2tKUQC").as_deref(),
        Some("4uLU6hMCjMI75M1A2tKUQC")
    );
    assert_eq!(
        track_id("spotify:track:4uLU6hMCjMI75M1A2tKUQC").as_deref(),
        Some("4uLU6hMCjMI75M1A2tKUQC")
    );
    assert_eq!(track_id("queen"), None);
}

#[test]
fn a_context_is_read_page_by_page() {
    let first: serde_json::Value = serde_json::from_str(
            r#"{"pages":[{"tracks":[{"uri":"spotify:track:a"},{"uri":"spotify:episode:x"}],"next_page_url":"hm://context-resolve/v1/next"}]}"#,
        )
        .unwrap();
    let mut uris = Vec::new();
    track_uris(&first, &mut uris);
    assert_eq!(uris, ["spotify:track:a"]);
    assert_eq!(next_page(&first), Some("hm://context-resolve/v1/next"));

    let page: serde_json::Value =
        serde_json::from_str(r#"{"tracks":[{"uri":"spotify:track:b"}]}"#).unwrap();
    track_uris(&page, &mut uris);
    assert_eq!(uris, ["spotify:track:a", "spotify:track:b"]);
    assert_eq!(next_page(&page), None);
}

#[test]
fn a_radio_seed_is_the_bare_track() {
    for id in [
        "spotify:radio:track:abc",
        "spotify:station:track:abc",
        "spotify:track:abc",
        "spotify:radio:track:spotify:track:abc",
        "abc",
    ] {
        assert_eq!(seed(id), "abc", "{id}");
    }
}

#[test]
fn a_context_track_without_a_uri_is_read_from_its_gid() {
    use base64::Engine as _;
    let gid = base64::engine::general_purpose::STANDARD.encode([0u8; 16]);
    let value: serde_json::Value =
        serde_json::from_str(&format!(r#"{{"tracks":[{{"gid":"{gid}"}}]}}"#)).unwrap();
    let mut uris = Vec::new();
    track_uris(&value, &mut uris);
    assert_eq!(uris, ["spotify:track:0000000000000000000000"]);
}

#[test]
fn a_collection_length_is_read_from_the_list() {
    let mut content = SelectedListContent::new();
    content.length = Some(42);
    let bytes = protobuf::Message::write_to_bytes(&content).unwrap();
    assert_eq!(length(&bytes), 42);
    assert_eq!(length(b"not a list"), 0);
}

#[test]
fn authorization_url_carries_pkce_and_scopes() {
    let url = authorization_url("http://127.0.0.1:8898/login", "challenge", "state");
    assert!(url.starts_with(AUTH_URL));
    assert!(url.contains("code_challenge=challenge"));
    assert!(url.contains("code_challenge_method=S256"));
    assert!(url.contains("state=state"));
    assert!(url.contains("playlist-read-private"));
    assert!(url.contains("streaming"));
}

#[test]
fn callback_ignores_preconnect_and_favicon() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = listener.local_addr().unwrap().port();
    listener.set_nonblocking(true).unwrap();

    let state = "state123".to_owned();
    let handle = std::thread::spawn(move || wait_for_code(&listener, &state));

    let _ = std::net::TcpStream::connect(("127.0.0.1", port));
    std::thread::sleep(Duration::from_millis(80));

    send(port, "GET /favicon.ico HTTP/1.1\r\nHost: localhost\r\n\r\n");
    std::thread::sleep(Duration::from_millis(80));

    send(
        port,
        "GET /login?code=code123&state=state123 HTTP/1.1\r\nHost: localhost\r\n\r\n",
    );

    let code = handle.join().unwrap().unwrap();
    assert_eq!(code, "code123");
}

#[test]
fn success_response_closes_the_tab() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = listener.local_addr().unwrap().port();
    listener.set_nonblocking(true).unwrap();

    let state = "state123".to_owned();
    let handle = std::thread::spawn(move || wait_for_code(&listener, &state));

    let response = send(
        port,
        "GET /login?code=code123&state=state123 HTTP/1.1\r\nHost: localhost\r\n\r\n",
    );

    handle.join().unwrap().unwrap();
    assert!(response.contains("Login complete"), "{response}");
    assert!(response.contains("window.close()"), "{response}");
}

#[test]
fn page_matches_the_platform() {
    assert!(page(200, true).contains("scheme=jondo"));
    assert!(page(200, true).contains("package=com.jondo.jondo"));
    assert!(page(200, false).contains("window.close()"));
    assert!(!page(200, false).contains("intent://"));
    assert!(page(400, true).contains("Login failed"));
    assert!(page(400, false).contains("Login failed"));
}

#[test]
fn the_session_keeps_jotas_shape() {
    let payload = r#"{"data":"0a0b0c","username":"tifieme"}"#;
    let auth: SpotifyAuth = serde_json::from_str(payload).unwrap();
    assert!(auth.signed_in());
    assert_eq!(from_hex(&auth.data).unwrap(), vec![0x0a, 0x0b, 0x0c]);
    assert_eq!(to_hex(&[0x0a, 0x0b, 0x0c]), "0a0b0c");
    assert_eq!(serde_json::to_string(&auth).unwrap(), payload);
}

fn send(port: u16, request: &str) -> String {
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream.set_read_timeout(Some(Duration::from_secs(1))).ok();
    stream.write_all(request.as_bytes()).unwrap();
    let mut response = Vec::new();
    let mut buffer = [0u8; 1024];
    while let Ok(read) = stream.read(&mut buffer) {
        if read == 0 {
            break;
        }

        response.extend_from_slice(&buffer[..read]);
    }

    String::from_utf8_lossy(&response).into_owned()
}

#[cfg(test)]
mod reuse_tests {
    use super::*;

    #[test]
    fn repeated_tracks_are_resolved_once_and_reused() {
        let uris: Vec<String> = (0..50_000)
            .map(|index| format!("spotify:track:{}", index % 2))
            .collect();
        assert_eq!(dedupe(&uris).len(), 2);

        let found = std::collections::HashMap::from([(
            "spotify:track:0".to_owned(),
            Song {
                id: "spotify:track:0".to_owned(),
                name: "Known".to_owned(),
                ..Default::default()
            },
        )]);

        let songs = reuse(&uris, &found);
        assert_eq!(songs.len(), uris.len());
        for (index, song) in songs.iter().enumerate() {
            if index % 2 == 0 {
                assert_eq!(song.name, "Known");
            } else {
                assert!(song.broken);
            }
        }
    }
}

#[cfg(test)]
mod base62_tests {
    use super::*;

    #[test]
    fn spotify_ids_roundtrip_through_hex() {
        for id in [
            "0VjIjW4GlUZAMYd2vXMi3b",
            "06HL4z0CvFAxyc27GXpf02",
            "3n3Ppam7vgaVa1iaRUc9Lp",
        ] {
            assert_eq!(hex_to_base62(&id_to_hex(id)), id, "{id}");
        }
    }
}

#[cfg(test)]
mod android_tests {
    use super::*;

    /// The success page hands the browser back to the app through a deep link.
    /// A scheme the manifest does not register strands the user in Chrome with
    /// the code already exchanged, which reads as "the login does nothing".
    #[test]
    fn the_browser_returns_to_the_package_that_declares_the_scheme() {
        const MANIFEST: &str =
            include_str!("../../../../../android/app/src/main/AndroidManifest.xml");
        const GRADLE: &str = include_str!("../../../../../android/app/build.gradle.kts");

        assert!(ANDROID_SUCCESS_PAGE.contains("scheme=jondo;"));
        assert!(MANIFEST.contains(r#"android:scheme="jondo""#));
        assert!(ANDROID_SUCCESS_PAGE.contains("package=com.jondo.jondo;"));
        assert!(GRADLE.contains(r#"applicationId = "com.jondo.jondo""#));
    }
}
