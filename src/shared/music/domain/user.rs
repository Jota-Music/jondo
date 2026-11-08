use super::Playlist;

#[derive(Clone, Debug, Default)]
pub struct UserView {
    pub username: String,
    pub name: String,
    pub image: Option<String>,
    pub playlists: Vec<Playlist>,
}
