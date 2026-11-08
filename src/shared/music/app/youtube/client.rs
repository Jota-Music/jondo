use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::shared::music::domain::{Channel, Playlist, Source};

use super::*;
use super::{cookies::*, extract_playlists::*, extract_videos::*};
use crate::shared::kernel::app::mutex;

#[derive(Clone, Copy)]
pub(super) struct Client {
    pub(super) name: &'static str,
    pub(super) version: &'static str,
    pub(super) number: u32,
    pub(super) agent: &'static str,
    pub(super) device_make: &'static str,
    pub(super) device_model: &'static str,
    pub(super) os_name: &'static str,
    pub(super) os_version: &'static str,
}

/// The client that carries a browser session. The others exist to resolve
/// streams and searches anonymously; only `WEB` answers with the account's own
/// library when the cookies are in the request.
pub(super) const WEB: Client = Client {
    name: "WEB",
    version: "2.20241126.01.00",
    number: 1,
    agent: BROWSER_AGENT,
    device_make: "",
    device_model: "",
    os_name: "",
    os_version: "",
};

pub(super) const CLIENTS: [Client; 3] = [
    Client {
        name: "VISIONOS",
        version: "1.02",
        number: 101,
        agent: "Mozilla/5.0 (Macintosh; Intel Mac OS X 15_7_3) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/26.0 Safari/605.1.15",
        device_make: "Apple",
        device_model: "RealityDevice17,1",
        os_name: "visionOS",
        os_version: "26.5.23O471",
    },
    Client {
        name: "ANDROID_VR",
        version: "1.62.27",
        number: 28,
        agent: "com.google.android.apps.youtube.vr.oculus/1.62.27 (Linux; U; Android 12; GB) gzip",
        device_make: "Oculus",
        device_model: "Quest 3",
        os_name: "Android",
        os_version: "12",
    },
    Client {
        name: "IOS",
        version: "20.10.4",
        number: 5,
        agent: "com.google.ios.youtube/20.10.4 (iPhone16,2; U; CPU iOS 18_3_2 like Mac OS X)",
        device_make: "Apple",
        device_model: "iPhone16,2",
        os_name: "iOS",
        os_version: "18.3.2.22D82",
    },
];

pub use crate::shared::music::domain::{Stream, Video};

#[derive(Default)]
pub(super) struct Visitor {
    pub(super) id: String,
    pub(super) key: String,
}

#[derive(Default)]
pub(super) struct Session {
    pub(super) cookies: String,
}

pub struct Youtube {
    pub(super) http: reqwest::Client,
    pub(super) visitor: Mutex<Option<Visitor>>,
    pub(super) session: Mutex<Option<Session>>,
    pub(super) streams: Mutex<HashMap<String, (Stream, Instant)>>,
}

impl Default for Youtube {
    fn default() -> Self {
        Self::new()
    }
}

impl Youtube {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(20))
                .build()
                .unwrap_or_default(),
            visitor: Mutex::new(None),
            session: Mutex::new(None),
            streams: Mutex::new(HashMap::new()),
        }
    }

    pub fn set_session(&self, cookies: &str) {
        let cookies = cookies.trim();
        *mutex::lock(&self.session) = (!cookies.is_empty()).then(|| Session {
            cookies: cookies.to_owned(),
        });
    }

    pub fn signed_in(&self) -> bool {
        mutex::lock(&self.session).is_some()
    }

    pub fn full(&self) -> bool {
        mutex::lock(&self.session)
            .as_ref()
            .is_some_and(|session| full(&session.cookies))
    }

    pub async fn search(&self, query: &str) -> Result<Vec<Video>, String> {
        let query = query.trim();
        if query.is_empty() {
            return Ok(Vec::new());
        }

        if let Some(video_id) = video_id_from_query(query) {
            let video = self.video(&video_id).await?;
            return Ok(vec![video]);
        }

        let mut last_error = None;
        for client in CLIENTS {
            let payload = json!({
                "query": query,
                "context": {"client": self.context(client)},
                "contentCheckOk": true,
                "racyCheckOk": true,
            });
            match self.post(client, WEB_SEARCH_URL, payload, false).await {
                Ok(value) => return Ok(extract_videos(&value)),
                Err(error) => last_error = Some(error),
            }
        }

        Err(last_error.unwrap_or_else(|| "YouTube search failed".to_owned()))
    }

    pub async fn search_channels(&self, query: &str) -> Result<Vec<Channel>, String> {
        let query = query.trim();
        if query.is_empty() {
            return Ok(Vec::new());
        }

        let mut last_error = None;
        for client in CLIENTS {
            let payload = json!({
                "query": query,
                "context": {"client": self.context(client)},
                "params": CHANNEL_FILTER,
                "contentCheckOk": true,
                "racyCheckOk": true,
            });
            match self.post(client, WEB_SEARCH_URL, payload, false).await {
                Ok(value) => return Ok(extract_channels(&value)),
                Err(error) => last_error = Some(error),
            }
        }

        Err(last_error.unwrap_or_else(|| "YouTube channel search failed".to_owned()))
    }

    pub async fn search_playlists(&self, query: &str) -> Result<Vec<Playlist>, String> {
        let query = query.trim();
        if query.is_empty() {
            return Ok(Vec::new());
        }

        let mut last_error = None;
        for client in CLIENTS {
            let payload = json!({
                "query": query,
                "context": {"client": self.context(client)},
                "params": PLAYLIST_FILTER,
                "contentCheckOk": true,
                "racyCheckOk": true,
            });
            match self.post(client, WEB_SEARCH_URL, payload, false).await {
                Ok(value) => return Ok(extract_playlists(&value)),
                Err(error) => last_error = Some(error),
            }
        }

        Err(last_error.unwrap_or_else(|| "YouTube playlist search failed".to_owned()))
    }

    pub async fn library(&self) -> Result<Vec<Playlist>, String> {
        if !self.full() {
            return Err("YouTube asks for a signed-in session".to_owned());
        }

        let origin = origin(LIBRARY_PAGE);
        let mut request = self
            .http
            .get(LIBRARY_PAGE)
            .header(reqwest::header::USER_AGENT, BROWSER_AGENT)
            .header(reqwest::header::ACCEPT_LANGUAGE, "en-US,en;q=0.9");

        if let Some(session) = mutex::lock(&self.session).as_ref()
            && let Some(hash) = sapisid_hash(&session.cookies, &origin)
        {
            request = request
                .header(reqwest::header::COOKIE, &session.cookies)
                .header("X-Goog-AuthUser", "0")
                .header("X-Origin", &origin)
                .header(reqwest::header::AUTHORIZATION, hash);
        }

        let response = request.send().await.map_err(|error| error.to_string())?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!("YouTube answered {status}"));
        }

        let html = response.text().await.map_err(|error| error.to_string())?;
        let data = initial_data(&html).ok_or_else(|| "YouTube answered no playlists".to_owned())?;
        if data.get("contents").is_none() {
            return Err(EXPIRED.to_owned());
        }

        Ok(extract_library(&data))
    }

    pub async fn playlist(&self, id: &str) -> Result<Playlist, String> {
        let id = id
            .trim()
            .strip_prefix("youtube:playlist:")
            .unwrap_or(id.trim())
            .to_owned();
        if id.is_empty() {
            return Err("playlist is required".to_owned());
        }

        let mut songs = Vec::new();
        let mut seen = HashSet::new();
        let mut name = String::new();
        let mut owner = String::new();
        let mut owner_id = None;
        let mut cover = None;
        let mut continuation: Option<String> = None;
        let mut previous = None;

        let signed = self.signed_in();
        let client = if signed { WEB } else { CLIENTS[0] };

        for page in 0..10 {
            let payload = if let Some(token) = &continuation {
                json!({
                    "context": {"client": self.context(client)},
                    "continuation": token,
                })
            } else {
                json!({
                    "context": {"client": self.context(client)},
                    "browseId": format!("VL{id}"),
                })
            };

            let value = match self.post(client, BROWSE_URL, payload.clone(), signed).await {
                Ok(value) => value,
                Err(error) if page == 0 && signed => {
                    match self.post(CLIENTS[0], BROWSE_URL, payload, false).await {
                        Ok(value) => value,
                        Err(_) => return Err(error),
                    }
                }
                Err(error) if page == 0 => return Err(error),
                Err(_) => break,
            };

            if name.is_empty() {
                name = value
                    .pointer("/header/playlistHeaderRenderer/title")
                    .and_then(renderer_text)
                    .or_else(|| {
                        value
                            .pointer("/metadata/playlistMetadataRenderer/title")
                            .and_then(Value::as_str)
                            .map(str::to_owned)
                    })
                    .or_else(|| {
                        value
                            .pointer("/header/pageHeaderRenderer/pageTitle")
                            .and_then(Value::as_str)
                            .map(str::to_owned)
                    })
                    .unwrap_or_default();
            }

            if owner.is_empty() {
                owner = value
                    .pointer("/header/playlistHeaderRenderer/ownerText")
                    .and_then(renderer_text)
                    .unwrap_or_default();
            }

            if owner_id.is_none() {
                owner_id = value
                    .pointer("/header/playlistHeaderRenderer/ownerText")
                    .and_then(browse_id);
            }

            if cover.is_none() {
                cover = value
                    .pointer(
                        "/header/playlistHeaderRenderer/banner/heroPlaylistThumbnailRenderer/thumbnail",
                    )
                    .and_then(|thumbnail| thumbnails(Some(thumbnail)));
            }

            let before = songs.len();
            collect_playlist_videos(&value, &owner, &name, &mut songs, &mut seen);
            if songs.len() == before && page > 0 {
                break;
            }

            continuation = find_continuation(&value);
            if continuation.is_none() || continuation == previous {
                break;
            }

            previous = continuation.clone();
        }

        if name.is_empty() {
            name = id.clone();
        }

        if cover.is_none() {
            cover = songs.first().and_then(|song| song.cover.clone());
        }

        for song in &mut songs {
            song.playlist_id = Some(id.clone());
        }

        let count = songs.len() as u32;
        Ok(Playlist {
            id: format!("youtube:playlist:{id}"),
            name,
            subtitle: (!owner.is_empty()).then(|| owner.clone()),
            cover,
            covers: Vec::new(),
            source: Source::YouTube,
            owner: (!owner.is_empty()).then_some(owner),
            owner_id,
            songs,
            track_count: count,
            loading: false,
        })
    }
}
