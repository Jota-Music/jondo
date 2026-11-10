mod catalog;
mod library;
mod login;
mod message;
mod playlist;
mod radio;
mod session;
#[cfg(test)]
#[path = "spotify.test.rs"]
mod tests;
mod tracks;

use std::time::Duration;

pub use catalog::{album, artist, search};
pub use library::{following, friends, library, liked, user};
pub use login::{Login, login};
pub use message::enrich_albums;
pub use playlist::playlist;
pub use radio::radio;

pub(crate) use playlist::{hex_to_base62, id_to_hex};
pub use session::Session;

/// The device id Spotify sees. It survives launches, because a new one registers
/// a new device on the account.
pub fn device() -> String {
    let mut bytes = [0u8; 20];
    rand::RngCore::fill_bytes(&mut rand::rng(), &mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub const CLIENT_ID: &str = "65b708073fc0480ea92a077233ca87bd";
const IMAGE_HOST: &str = "https://i.scdn.co/image/";
const SPCLIENT: &str = "https://spclient.wg.spotify.com";
const LIKED_COVER: &str = "https://misc.scdn.co/liked-songs/liked-songs-640.png";
const AUTH_URL: &str = "https://accounts.spotify.com/authorize";
const TOKEN_URL: &str = "https://accounts.spotify.com/api/token";
const LOGIN_TIMEOUT: Duration = Duration::from_secs(5 * 60);

const SCOPES: &[&str] = &[
    "playlist-read",
    "playlist-read-collaborative",
    "playlist-read-private",
    "streaming",
    "user-library-read",
    "user-read-private",
];
