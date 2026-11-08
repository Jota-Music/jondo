#[cfg(not(target_os = "android"))]
use std::path::PathBuf;
use std::sync::Arc;

use crate::music::rooms::{self};
use crate::shared::music::domain::{
    Album, ArtistView, Channel, ChannelView, Follow, Playlist, Song, UserView,
};

use crate::shared::spotify;
use crate::shared::youtube::{Stream, Video};

pub enum Event {
    SpotifyLogin(Result<spotify::Login, String>),
    SpotifyResumed(Arc<spotify::Session>),
    SpotifyLibrary(Result<Vec<Playlist>, String>),
    SpotifyPlaylist(String, Result<Playlist, String>),
    SpotifyAlbum(String, Result<Album, String>),
    SpotifyArtist(String, Result<ArtistView, String>),
    SpotifyDiscography(String, String, Result<Vec<Album>, String>),
    SpotifyCatalog(
        String,
        Result<crate::shared::music::domain::SpotifySearch, String>,
    ),
    SpotifyFollowing(Result<Vec<Follow>, String>),
    SpotifyFriends(Result<Vec<String>, String>),
    SpotifyUser(String, Result<UserView, String>),
    SpotifyFailed(String),
    YouTubeSearch(String, Result<Vec<Video>, String>),
    YouTubeLibrary(u64, Result<Vec<Playlist>, String>),
    YouTubeVideos(String, Result<Vec<Video>, String>),
    YouTubeLink {
        id: String,
        video: Option<String>,
    },
    YouTubePreview {
        token: u64,
        result: Result<Stream, String>,
    },
    YouTubeChannels(String, Result<Vec<Channel>, String>),
    YouTubeChannel(String, Result<ChannelView, String>),
    YouTubePlaylists(String, Result<Vec<Playlist>, String>),
    YouTubePlaylist(String, Result<Playlist, String>),
    YouTubeStream {
        token: u64,
        song: Song,
        start: f64,
        play: bool,
        result: Result<Stream, String>,
    },
    ClipboardSong(Result<Song, String>),
    Room(Box<rooms::Event>),
    Update(Result<crate::settings::app::update::Info, String>),
    /// A finished background backup: the revision it started from, the store
    /// it worked on, and its result.
    Backup(
        u64,
        Box<crate::shared::storage::Store>,
        Result<crate::settings::app::backup::Stats, String>,
    ),
    Installed(Result<std::path::PathBuf, String>),
    Browsers(Vec<&'static str>),
    BrowserCookies(&'static str, Result<String, String>),
    /// A background re-read of the remembered browser. The flag marks the read
    /// that is retrying after YouTube answered 401, which must report failure.
    BrowserRefresh(bool, Result<String, String>),
    /// The browser that holds the stored session, found by its `SAPISID`, so
    /// launching with a pasted session can start refreshing it.
    Rescue(Option<(&'static str, String)>),
    #[cfg(not(target_os = "android"))]
    Folder(Option<PathBuf>),
    #[cfg(not(target_os = "android"))]
    ExportPath(Option<PathBuf>),
    #[cfg(not(target_os = "android"))]
    ImportPath(Option<PathBuf>),
    CookiesFile(Result<String, String>),
}

pub struct Jobs {
    runtime: tokio::runtime::Runtime,
    sender: tokio::sync::mpsc::UnboundedSender<Event>,
    pub(super) receiver: tokio::sync::mpsc::UnboundedReceiver<Event>,
    pub(super) notify: Arc<tokio::sync::Notify>,
}

impl Jobs {
    pub(super) fn new() -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            // The blocking pool defaults to 512 threads. File dialogs, backups
            // and browser scans need a handful, and every extra thread is a
            // stack plus its own allocator arena.
            .max_blocking_threads(8)
            .enable_all()
            .build()
            .expect("tokio runtime");
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        Self {
            runtime,
            sender,
            receiver,
            notify: Arc::new(tokio::sync::Notify::new()),
        }
    }

    pub(crate) fn spawn<F>(&self, future: F)
    where
        F: std::future::Future<Output = Event> + Send + 'static,
    {
        let sender = self.sender.clone();
        let notify = self.notify.clone();
        self.runtime.spawn(async move {
            let event = future.await;
            let _ = sender.send(event);
            notify.notify_one();
        });
    }

    pub(crate) fn blocking<F>(&self, task: F)
    where
        F: FnOnce() -> Event + Send + 'static,
    {
        let sender = self.sender.clone();
        let notify = self.notify.clone();
        self.runtime.spawn_blocking(move || {
            let event = task();
            let _ = sender.send(event);
            notify.notify_one();
        });
    }

    pub(crate) fn run<F>(&self, future: F)
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        self.runtime.spawn(future);
    }

    /// Work that blocks a thread (files, dialogs) without holding a frame.
    pub(crate) fn task<F>(&self, task: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.runtime.spawn_blocking(task);
    }

    pub(crate) fn emit(&self) -> impl Fn(Event) + Clone + Send + 'static {
        let sender = self.sender.clone();
        let notify = self.notify.clone();
        move |event| {
            let _ = sender.send(event);
            notify.notify_one();
        }
    }
}
