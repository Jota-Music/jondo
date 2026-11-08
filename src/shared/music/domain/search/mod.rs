use serde::{Deserialize, Serialize};

use super::Song;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Filter {
    All,
    YouTube,
    Spotify,
    Local,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Order {
    #[default]
    Added,
    Reverse,
    DurationAsc,
    DurationDesc,
}

impl Order {
    pub const ALL: [Self; 4] = [
        Self::Added,
        Self::Reverse,
        Self::DurationAsc,
        Self::DurationDesc,
    ];
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SearchType {
    #[default]
    Track,
    User,
    Album,
    Playlist,
    Artist,
}

impl SearchType {
    pub const ALL: [Self; 5] = [
        Self::Track,
        Self::User,
        Self::Album,
        Self::Playlist,
        Self::Artist,
    ];
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SpotifySearch {
    pub tracks: Vec<Song>,
}

/// Whether the rows are the playlist exactly as it stores them. A filter or a
/// sort changes what a row index means, so only an unmodified list can move a
/// song without shifting another one by accident.
pub fn reorderable(local: bool, all: &[Song], ids: &[String]) -> bool {
    local && all.len() > 1 && all.iter().map(|song| &song.id).eq(ids)
}

/// Indices of the songs a playlist page shows, in the requested order.
pub fn visible(all: &[Song], query: &str, order: Order, searchable: bool) -> Vec<usize> {
    let query = query.trim().to_lowercase();
    let mut rows: Vec<usize> = all
        .iter()
        .enumerate()
        .filter(|(_, song)| {
            !searchable
                || query.is_empty()
                || song.name.to_lowercase().contains(&query)
                || song.artists.join(" ").to_lowercase().contains(&query)
        })
        .map(|(index, _)| index)
        .collect();

    if !searchable {
        return rows;
    }

    match order {
        Order::Added => {}
        Order::Reverse => rows.reverse(),
        Order::DurationAsc => rows.sort_by_key(|index| all[*index].duration),
        Order::DurationDesc => rows.sort_by_key(|index| std::cmp::Reverse(all[*index].duration)),
    }

    rows
}

#[cfg(test)]
#[path = "search.test.rs"]
mod tests;
