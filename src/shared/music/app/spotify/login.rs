use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::time::{Duration, Instant};

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use librespot_core::authentication::Credentials;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::shared::crypto::hold::Hold;
use crate::shared::storage::SpotifyAuth;

use super::session::*;
use super::*;

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    username: Option<String>,
}

pub struct Login {
    pub session: Arc<Session>,
    pub auth: SpotifyAuth,
}

/// Signs in the way Jota does: the browser answer buys one access token, and the
/// accesspoint turns it into reusable credentials. From then on the session is
/// renewed through login5, so nothing here ever asks Spotify to refresh an OAuth
/// token — that grant is revoked as soon as it is spent.
pub async fn login(device: String) -> Result<Login, String> {
    let _hold = Hold::start();
    let (code, verifier, redirect) = tokio::task::spawn_blocking(browser)
        .await
        .map_err(|error| error.to_string())??;
    let access = exchange(&code, &verifier, &redirect).await?;
    let client = open(&device, Credentials::with_access_token(access)).await?;
    let auth = SpotifyAuth {
        username: client.username(),
        data: to_hex(&client.auth_data()),
    };

    if !auth.signed_in() {
        return Err("Spotify returned no reusable credentials".to_owned());
    }

    log::info!("spotify: signed in as {}", auth.username);
    Ok(Login {
        session: Session::wrap(client),
        auth,
    })
}

pub(super) fn browser() -> Result<(String, String, String), String> {
    let verifier = random_b64(64);
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let state = random_hex(16);

    let (listener, port) = bind_listener()?;
    let redirect = format!("http://127.0.0.1:{port}/login");
    let url = authorization_url(&redirect, &challenge, &state);

    log::info!("spotify: open this URL if the browser does not open: {url}");
    if let Err(error) = open_browser(&url) {
        log::warn!("spotify: cannot open the browser: {error}");
        return Err(format!(
            "cannot open the browser. Open this URL manually: {url}"
        ));
    }

    let code = wait_for_code(&listener, &state)?;
    log::info!("spotify: authorization code received");
    Ok((code, verifier, redirect))
}

pub(super) fn authorization_url(redirect: &str, challenge: &str, state: &str) -> String {
    let mut url = url::Url::parse(AUTH_URL).expect("valid authorize url");
    url.query_pairs_mut()
        .append_pair("client_id", CLIENT_ID)
        .append_pair("response_type", "code")
        .append_pair("redirect_uri", redirect)
        .append_pair("code_challenge_method", "S256")
        .append_pair("code_challenge", challenge)
        .append_pair("state", state)
        .append_pair("scope", &SCOPES.join(" "));
    url.to_string()
}

pub(super) fn bind_listener() -> Result<(TcpListener, u16), String> {
    let mut last = String::new();
    for port in 8898..=8908 {
        match TcpListener::bind(("127.0.0.1", port)) {
            Ok(listener) => {
                listener
                    .set_nonblocking(true)
                    .map_err(|error| format!("spotify listener: {error}"))?;
                log::info!("spotify: callback listening on 127.0.0.1:{port}");
                return Ok((listener, port));
            }
            Err(error) => last = format!("127.0.0.1:{port}: {error}"),
        }
    }

    Err(format!("cannot open the Spotify callback port ({last})"))
}

pub(super) fn wait_for_code(listener: &TcpListener, state: &str) -> Result<String, String> {
    let deadline = Instant::now() + LOGIN_TIMEOUT;

    while Instant::now() < deadline {
        match listener.accept() {
            Ok((mut stream, _)) => {
                stream.set_read_timeout(Some(Duration::from_secs(5))).ok();
                let mut buffer = [0u8; 8192];
                let read = match stream.read(&mut buffer) {
                    Ok(0) => continue,
                    Ok(read) => read,
                    Err(error)
                        if matches!(
                            error.kind(),
                            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                        ) =>
                    {
                        continue;
                    }
                    Err(_) => continue,
                };

                let request = String::from_utf8_lossy(&buffer[..read]);
                let path = request
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                    .unwrap_or("/");

                if !path.starts_with("/login") {
                    respond(&mut stream, 404);
                    continue;
                }

                let Ok(url) = url::Url::parse(&format!("http://127.0.0.1{path}")) else {
                    respond(&mut stream, 400);
                    continue;
                };

                let mut code = None;
                let mut returned_state = None;
                let mut returned_error = None;
                for (key, value) in url.query_pairs() {
                    match key.as_ref() {
                        "code" => code = Some(value.into_owned()),
                        "state" => returned_state = Some(value.into_owned()),
                        "error" => returned_error = Some(value.into_owned()),
                        _ => {}
                    }
                }

                if let Some(error) = returned_error {
                    respond(&mut stream, 400);
                    return Err(format!("Spotify authorization failed: {error}"));
                }

                if returned_state.as_deref() != Some(state) {
                    respond(&mut stream, 400);
                    continue;
                }

                let Some(code) = code else {
                    respond(&mut stream, 400);
                    continue;
                };

                respond(&mut stream, 200);
                return Ok(code);
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(error) => return Err(format!("spotify callback: {error}")),
        }
    }

    Err("Spotify sign-in timed out. Try again from Settings.".to_owned())
}

pub(super) fn open_browser(url: &str) -> Result<(), String> {
    crate::shared::platform::links::open(url)
}

pub(super) fn respond(stream: &mut std::net::TcpStream, status: u16) {
    let body = page(status, cfg!(target_os = "android"));
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        _ => "OK",
    };

    let response = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

pub(super) fn page(status: u16, mobile: bool) -> &'static str {
    match (status, mobile) {
        (200, true) => ANDROID_SUCCESS_PAGE,
        (200, false) => SUCCESS_PAGE,
        _ => FAILURE_PAGE,
    }
}

const SUCCESS_PAGE: &str = r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>Jondo</title>
<style>
  body{margin:0;font-family:system-ui,-apple-system,sans-serif;background:#0c0a09;color:#e4e7eb;display:grid;place-items:center;min-height:100vh}
  .card{text-align:center;padding:2rem;max-width:30rem}
  h1{color:#22c55e;margin:0 0 0.5rem;font-size:1.5rem;text-wrap:balance}
  p{color:#a1a1aa;margin:0.5rem 0;text-wrap:balance}
</style>
</head>
<body>
  <div class="card">
    <h1>Login complete</h1>
    <p>You can close this tab and return to Jondo.</p>
  </div>
  <script>
    setTimeout(function(){ window.close(); }, 800);
  </script>
</body>
</html>"#;

pub(super) const ANDROID_SUCCESS_PAGE: &str = r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>Jondo</title>
<style>
  body{margin:0;font-family:system-ui,-apple-system,sans-serif;background:#0c0a09;color:#e4e7eb;display:grid;place-items:center;min-height:100vh}
  .card{text-align:center;padding:2rem;max-width:30rem}
  h1{color:#22c55e;margin:0 0 0.5rem;font-size:1.5rem;text-wrap:balance}
  p{color:#a1a1aa;margin:0.5rem 0;text-wrap:balance}
</style>
</head>
<body>
  <div class="card">
    <h1>Login complete</h1>
    <p>Returning you to Jondo...</p>
  </div>
  <script>
    location.replace('intent://callback#Intent;scheme=jondo;package=com.jondo.jondo;end');
    setTimeout(function(){ window.close(); }, 800);
  </script>
</body>
</html>"#;

const FAILURE_PAGE: &str = r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>Jondo</title>
<style>
  body{margin:0;font-family:system-ui,-apple-system,sans-serif;background:#0c0a09;color:#e4e7eb;display:grid;place-items:center;min-height:100vh}
  .card{text-align:center;padding:2rem;max-width:30rem}
  h1{color:#ef4444;margin:0 0 0.5rem;font-size:1.5rem;text-wrap:balance}
  p{color:#a1a1aa;margin:0.5rem 0;text-wrap:balance}
</style>
</head>
<body>
  <div class="card">
    <h1>Login failed</h1>
    <p>Jondo could not complete the Spotify login. You can close this tab and try again.</p>
  </div>
</body>
</html>"#;

pub(super) async fn exchange(code: &str, verifier: &str, redirect: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|error| format!("spotify token: {error}"))?;
    let response = client
        .post(TOKEN_URL)
        .form(&[
            ("client_id", CLIENT_ID),
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect),
            ("code_verifier", verifier),
        ])
        .send()
        .await
        .map_err(|error| format!("spotify token: {error}"))?;

    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    if !status.is_success() {
        log::warn!("spotify token exchange failed ({status}): {text}");
        return Err(format!("spotify token exchange failed ({status}): {text}"));
    }

    let token: TokenResponse =
        serde_json::from_str(&text).map_err(|error| format!("spotify token response: {error}"))?;
    log::info!(
        "spotify: token exchanged for {}",
        token.username.unwrap_or_default()
    );
    Ok(token.access_token)
}
