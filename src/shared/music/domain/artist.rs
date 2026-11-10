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

/// The groups Spotify returns, in the order the artist page shows them.
pub const ALBUM_GROUPS: [&str; 4] = ["album", "single", "compilation", "appears_on"];

/// Every group an artist page needs, including the ones with no album.
pub fn groups(albums: &[Album]) -> Vec<String> {
    let mut groups: Vec<String> = ALBUM_GROUPS.iter().map(|key| (*key).to_owned()).collect();
    for album in albums {
        let group = if album.group.trim().is_empty() {
            "album"
        } else {
            album.group.trim()
        };

        let group = group.to_lowercase();
        if !groups.contains(&group) {
            groups.push(group);
        }
    }

    groups
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
