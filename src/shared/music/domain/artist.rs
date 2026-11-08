use serde::{Deserialize, Serialize};

use super::Song;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ArtistRef {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Album {
    pub id: String,
    pub name: String,
    pub artists: Vec<ArtistRef>,
    pub cover: Option<String>,
    pub year: u32,
    pub group: String,
    pub tracks: Vec<Song>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ArtistView {
    pub id: String,
    pub name: String,
    pub image: Option<String>,
    pub followers: u64,
    pub genres: Vec<String>,
    pub tracks: Vec<Song>,
    pub albums: Vec<Album>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ArtistSummary {
    pub id: String,
    pub name: String,
    pub image: Option<String>,
}
