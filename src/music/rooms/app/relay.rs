use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use url::Url;

use crate::music::rooms::domain::{RelayCheck, RoomStatus, TokenState};
use crate::shared::i18n::t;

pub enum Command {
    Send(String),
    Stop,
}

pub fn ws_url(raw: &str, room: &str, role: &str) -> Result<Url, String> {
    let room = room.trim();
    if room.is_empty() {
        return Err("room code is empty".to_owned());
    }

    let mut url = normalize(raw)?;
    if url.path().is_empty() || url.path() == "/" {
        url.set_path("/ws");
    }

    {
        let mut query = url.query_pairs_mut();
        query.append_pair("room", room);
        if !role.is_empty() {
            query.append_pair("role", role);
        }
    }

    Ok(url)
}

pub fn health_url(raw: &str) -> Result<Url, String> {
    let mut url = normalize(raw)?;
    url.set_scheme(if url.scheme() == "wss" {
        "https"
    } else {
        "http"
    })
    .map_err(|_| "unsupported relay url scheme".to_owned())?;
    url.set_path("/healthz");
    url.set_query(None);
    Ok(url)
}

pub fn status_url(raw: &str, room: &str) -> Result<Url, String> {
    let room = room.trim();
    if room.is_empty() {
        return Err("room code is empty".to_owned());
    }

    let mut url = normalize(raw)?;
    url.set_scheme(if url.scheme() == "wss" {
        "https"
    } else {
        "http"
    })
    .map_err(|_| "unsupported relay url scheme".to_owned())?;
    {
        let mut path = url
            .path_segments_mut()
            .map_err(|_| "invalid relay url".to_owned())?;
        path.clear().push("rooms").push(room);
    }

    url.set_query(None);
    Ok(url)
}

pub fn normalize(raw: &str) -> Result<Url, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err("relay url is empty".to_owned());
    }

    let raw = if raw.contains("://") {
        raw.to_owned()
    } else {
        format!("wss://{raw}")
    };

    let mut url = Url::parse(&raw).map_err(|error| error.to_string())?;
    match url.scheme() {
        "http" => {
            let _ = url.set_scheme("ws");
        }
        "https" => {
            let _ = url.set_scheme("wss");
        }
        "ws" | "wss" => {}
        _ => return Err("unsupported relay url scheme".to_owned()),
    }

    Ok(url)
}

pub async fn check(client: &reqwest::Client, raw: &str, token: &str) -> RelayCheck {
    let mut verdict = RelayCheck::default();
    let target = match health_url(raw) {
        Ok(target) => target,
        Err(error) => {
            verdict.error = Some(error);
            return verdict;
        }
    };

    let mut request = client.get(target);
    if !token.is_empty() {
        request = request.bearer_auth(token);
    }

    let response = match request.send().await {
        Ok(response) => response,
        Err(error) => {
            log::warn!("relay: health: {error}");
            verdict.error = Some(
                t("Could not reach the relay. Check the address and your connection.").to_owned(),
            );
            return verdict;
        }
    };

    if !response.status().is_success() {
        verdict.error =
            Some(t("The relay answered {}.").replace("{}", &response.status().to_string()));
        return verdict;
    }

    let health: Health = response.json().await.unwrap_or_default();
    if !health.auth {
        verdict.token = TokenState::None;
        return verdict;
    }

    if token.is_empty() {
        verdict.token = TokenState::Required;
        return verdict;
    }

    verdict.token = match probe(client, raw, token).await {
        true => TokenState::Rejected,
        false => TokenState::Accepted,
    };

    verdict
}

#[derive(Default, serde::Deserialize)]
struct Health {
    #[serde(default)]
    auth: bool,
}

// An unknown code is answered without touching a room, so the status endpoint
// tells us what the relay thinks of the bearer without joining anything.
async fn probe(client: &reqwest::Client, raw: &str, token: &str) -> bool {
    let Ok(target) = status_url(raw, "jondo-probe") else {
        return false;
    };

    let Ok(response) = client.get(target).bearer_auth(token).send().await else {
        return false;
    };

    response.status() == reqwest::StatusCode::UNAUTHORIZED
}

pub async fn status(
    client: &reqwest::Client,
    raw: &str,
    code: &str,
    token: &str,
) -> Result<RoomStatus, String> {
    let target = status_url(raw, code)?;
    let mut request = client.get(target);
    if !token.is_empty() {
        request = request.bearer_auth(token);
    }

    let response = request.send().await.map_err(|error| {
        log::warn!("relay: status {code}: {error}");
        "Could not reach the relay.".to_owned()
    })?;
    if !response.status().is_success() {
        return Err(format!("The relay answered {}.", response.status()));
    }

    response
        .json()
        .await
        .map_err(|error| format!("Invalid room status: {error}"))
}

pub async fn run<F>(
    generation: u64,
    url: Url,
    token: String,
    password: String,
    mut commands: UnboundedReceiver<Command>,
    emit: F,
) where
    F: Fn(crate::music::rooms::domain::Event) + Send + 'static,
{
    let mut request = match url.as_str().into_client_request() {
        Ok(request) => request,
        Err(error) => {
            log::warn!("room: invalid request: {error}");
            emit(crate::music::rooms::domain::Event::Closed(generation, true));
            return;
        }
    };

    if !token.is_empty()
        && let Ok(value) = HeaderValue::from_str(&format!("Bearer {token}"))
    {
        request.headers_mut().insert("Authorization", value);
    }

    if !password.is_empty()
        && let Ok(value) = HeaderValue::from_str(&password)
    {
        request.headers_mut().insert("X-Room-Password", value);
    }

    let stream = tokio::time::timeout(
        Duration::from_secs(10),
        tokio_tungstenite::connect_async(request),
    )
    .await;
    let mut socket = match stream {
        Ok(Ok((socket, _))) => socket,
        Ok(Err(error)) => {
            let required = matches!(
                &error,
                tokio_tungstenite::tungstenite::Error::Http(response)
                    if response.status().as_u16() == 401
            );
            log::warn!("room: dial: {error}");
            emit(crate::music::rooms::domain::Event::Closed(
                generation, required,
            ));
            return;
        }
        Err(_) => {
            log::warn!("room: dial timed out");
            emit(crate::music::rooms::domain::Event::Closed(
                generation, false,
            ));
            return;
        }
    };

    emit(crate::music::rooms::domain::Event::Connected(generation));

    loop {
        tokio::select! {
            command = commands.recv() => {
                match command {
                    Some(Command::Send(text)) => {
                        if socket.send(Message::text(text)).await.is_err() {
                            break;
                        }
                    }
                    Some(Command::Stop) | None => break,
                }
            }
            message = socket.next() => {
                match message {
                    Some(Ok(Message::Text(text))) => emit(crate::music::rooms::domain::Event::Message(generation, text.to_string())),
                    Some(Ok(Message::Ping(payload))) => {
                        let _ = socket.send(Message::Pong(payload)).await;
                    }
                    Some(Ok(Message::Close(_))) | Some(Err(_)) | None => break,
                    Some(Ok(_)) => {}
                }
            }
        }
    }

    let _ = socket.close(None).await;
    emit(crate::music::rooms::domain::Event::Closed(
        generation, false,
    ));
}

pub fn send(commands: &Option<UnboundedSender<Command>>, text: String) {
    if let Some(commands) = commands {
        let _ = commands.send(Command::Send(text));
    }
}
