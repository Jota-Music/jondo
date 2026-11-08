mod channel;
mod client;
mod cookies;
mod extract_playlists;
mod extract_videos;
mod stream;
#[cfg(test)]
#[path = "youtube.test.rs"]
mod tests;

pub use client::Youtube;
pub use client::{Stream, Video};
pub use cookies::{full, parse_cookies, same};

/// A logged-out YouTube answers the library page with a shell and a `200`, so
/// the round trip alone cannot tell a dead session from an empty account.
pub const EXPIRED: &str = "YouTube closed the session";
const PLAYER_URL: &str = "https://www.youtube.com/youtubei/v1/player";
/// The same host the player API talks to, so a search reuses the connection it
/// already opened, and it never answered slower than `music.youtube.com`.
const WEB_SEARCH_URL: &str = "https://www.youtube.com/youtubei/v1/search";
const BROWSE_URL: &str = "https://www.youtube.com/youtubei/v1/browse";
const RESOLVE_URL: &str = "https://www.youtube.com/youtubei/v1/navigation/resolve_url";
const CHANNEL_FILTER: &str = "EgIQAg==";
const CHANNEL_PLAYLISTS: &str = "EglwbGF5bGlzdHPyBgoKCEIGCgIQaCIA";
const PLAYLIST_FILTER: &str = "EgIQAw==";
const CHANNEL_VIDEOS: &str = "EgZ2aWRlb3PyBgQKAjoA";
const DEFAULT_KEY: &str = "AIzaSyAO_FJ2SlqU8Q4STEHLGCilw_Y9_11qcW8";
const HOMEPAGE: &str = "https://www.youtube.com/";
const LIBRARY_PAGE: &str = "https://www.youtube.com/feed/playlists";
const BROWSER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";
