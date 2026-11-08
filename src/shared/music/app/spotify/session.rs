use std::sync::Arc;
use std::time::Duration;

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use librespot_core::SessionConfig;
use librespot_core::authentication::Credentials;
use librespot_core::session::Session as Client;
use librespot_protocol::authentication::AuthenticationType;
use rand::RngCore;

use crate::shared::storage::SpotifyAuth;

/// A signed-in account. Requests ask it for a token every time; the accesspoint
/// credentials behind it never expire, so renewal is a login5 round trip instead
/// of the OAuth refresh Jota never performs either.
pub struct Session {
    client: Client,
    username: String,
}

impl Session {
    /// Adopts the pair Jota saves: the account and its reusable credentials.
    pub async fn connect(device: &str, auth: &SpotifyAuth) -> Result<Arc<Self>, String> {
        let data =
            from_hex(&auth.data).ok_or_else(|| "Spotify session is not valid hex".to_owned())?;
        let credentials = Credentials {
            username: Some(auth.username.clone()),
            auth_type: AuthenticationType::AUTHENTICATION_STORED_SPOTIFY_CREDENTIALS,
            auth_data: data,
        };

        open(device, credentials).await.map(Self::wrap)
    }

    pub(super) fn wrap(client: Client) -> Arc<Self> {
        Arc::new(Self {
            username: client.username(),
            client,
        })
    }

    pub fn username(&self) -> String {
        self.username.clone()
    }

    /// A login5 access token, renewed by the manager on its own clock.
    pub async fn access(&self) -> Result<String, String> {
        self.client
            .login5()
            .auth_token()
            .await
            .map(|token| token.access_token)
            .map_err(|error| format!("spotify session: {error}"))
    }
}

pub(super) async fn open(device: &str, credentials: Credentials) -> Result<Client, String> {
    let config = SessionConfig {
        device_id: device.to_owned(),
        ..Default::default()
    };

    let client = Client::new(config, None);
    client
        .connect(credentials, false)
        .await
        .map_err(|error| format!("spotify session: {error}"))?;
    Ok(client)
}

pub(super) fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub(super) fn from_hex(text: &str) -> Option<Vec<u8>> {
    let text = text.trim();
    if !text.len().is_multiple_of(2) {
        return None;
    }

    (0..text.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&text[at..at + 2], 16).ok())
        .collect()
}

/// Sends a request with the session's token. Transient answers are retried with
/// a growing wait, the way the original client treats them.
pub(super) async fn authorized<F>(
    session: &Session,
    label: &str,
    build: F,
) -> Result<reqwest::Response, String>
where
    F: Fn(&str) -> reqwest::RequestBuilder,
{
    let mut wait = Duration::from_millis(200);
    loop {
        let access = session.access().await?;
        let response = build(&access)
            .send()
            .await
            .map_err(|error| format!("{label}: {error}"))?;
        let status = response.status();
        if retryable(status) && wait <= RETRY_MAX {
            log::debug!("{label}: {status}, retrying in {wait:?}");
            tokio::time::sleep(wait).await;
            wait *= 4;
            continue;
        }

        return Ok(response);
    }
}

pub(super) fn retryable(status: reqwest::StatusCode) -> bool {
    matches!(status.as_u16(), 500 | 502 | 503 | 504)
}

const RETRY_MAX: Duration = Duration::from_secs(5);

pub(super) fn random_b64(len: usize) -> String {
    let mut bytes = vec![0u8; len];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

pub(super) fn random_hex(len: usize) -> String {
    let mut bytes = vec![0u8; len];
    rand::rng().fill_bytes(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
