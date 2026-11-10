mod spotify;
mod system;
mod youtube;

use super::*;

impl Jondo {
    pub fn drain_events(&mut self, cx: &mut gpui::Context<Self>) -> bool {
        let mut any = false;
        while let Ok(event) = self.jobs.receiver.try_recv() {
            any = true;
            match event {
                event @ (Event::SpotifyLogin(_)
                | Event::SpotifyResumed(_)
                | Event::SpotifyLibrary(_)
                | Event::SpotifyPlaylist(_, _)
                | Event::SpotifyAlbum(_, _)
                | Event::SpotifyArtist(_, _)
                | Event::SpotifyDiscography(_, _, _)
                | Event::SpotifyFollowing(_)
                | Event::SpotifyFriends(_)
                | Event::SpotifyUser(_, _)
                | Event::SpotifyCatalog(_, _)
                | Event::SpotifyFailed(_)) => self.spotify_event(event, cx),
                event @ (Event::YouTubeLibrary(_, _)
                | Event::YouTubeSearch(_, _)
                | Event::YouTubeVideos(_, _)
                | Event::YouTubeLink { .. }
                | Event::YouTubePreview { .. }
                | Event::YouTubeChannels(_, _)
                | Event::YouTubePlaylists(_, _)
                | Event::YouTubePlaylist(_, _)
                | Event::YouTubeChannel(_, _)
                | Event::YouTubeStream { .. }) => self.youtube_event(event, cx),
                event @ (Event::ClipboardSong(_)
                | Event::Room(_)
                | Event::Update(_)
                | Event::Installed(_)
                | Event::Browsers(_)
                | Event::BrowserCookies(_, _)
                | Event::Rescue(_)
                | Event::BrowserRefresh(_, _)
                | Event::CookiesFile(_)) => self.system_event(event, cx),
                #[cfg(not(target_os = "android"))]
                Event::Backup {
                    revision,
                    job,
                    store,
                    result,
                } => self.backup_now(job, revision, *store, result, cx),
                #[cfg(target_os = "android")]
                Event::Backup { .. } => {}
                #[cfg(target_os = "android")]
                Event::Staged {
                    revision,
                    uri,
                    purpose,
                    store,
                    result,
                } => self.staged(revision, uri, purpose, *store, result, cx),
                #[cfg(not(target_os = "android"))]
                Event::Folder(path) => {
                    if let Some(path) = path
                        && !path.as_os_str().is_empty()
                    {
                        self.adopt_folder(&path, cx);
                    }
                }
                #[cfg(not(target_os = "android"))]
                Event::ExportPath(path) => {
                    if let Some(path) = path {
                        self.write_backup(&path, cx);
                    }
                }
                #[cfg(not(target_os = "android"))]
                Event::ImportPath(path) => {
                    if let Some(path) = path {
                        self.read_backup(&path, cx);
                    }
                }
            }
        }

        any
    }
}
