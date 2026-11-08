use rand::seq::SliceRandom;
use rand::{SeedableRng, rngs::StdRng};
use std::collections::HashMap;

use crate::shared::music::domain::Song;

fn seeded(seed: u64) -> StdRng {
    StdRng::seed_from_u64(seed)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Repeat {
    Off,
    All,
    One,
}

impl Repeat {
    pub fn next(self) -> Self {
        match self {
            Self::Off => Self::All,
            Self::All => Self::One,
            Self::One => Self::Off,
        }
    }
}

#[derive(Default)]
pub struct Queue {
    pub songs: Vec<Song>,
    pub at: usize,
    pub shuffle: bool,
    pub revision: u64,
}

impl Queue {
    fn touch(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }

    pub fn set(&mut self, songs: Vec<Song>, start: usize) {
        self.songs = songs;
        self.at = start.min(self.songs.len().saturating_sub(1));
        self.rebuild();
    }

    // Replaces the queue with an order that is already authoritative, such as
    // one received from a room or restored from storage, so it must not be
    // reshuffled on arrival.
    pub fn replace(&mut self, songs: Vec<Song>, start: usize) {
        self.songs = songs;
        self.at = start.min(self.songs.len().saturating_sub(1));
        self.touch();
    }

    pub fn current(&self) -> Option<&Song> {
        self.songs.get(self.at)
    }

    /// Whether the queue holds exactly these songs, in any order. A source list
    /// matches a shuffled queue by membership, and duplicate ids are counted.
    pub fn same(&self, songs: &[Song]) -> bool {
        if self.songs.len() != songs.len() {
            return false;
        }

        let mut count: HashMap<&str, usize> = HashMap::with_capacity(self.songs.len());
        for song in &self.songs {
            *count.entry(song.id.as_str()).or_default() += 1;
        }

        for song in songs {
            let Some(left) = count.get_mut(song.id.as_str()) else {
                return false;
            };

            if *left == 0 {
                return false;
            }

            *left -= 1;
        }

        true
    }

    /// Replaces the track playing now without touching the rest of the queue,
    /// which is what a "play this" link does in Jota.
    pub fn replace_current(&mut self, song: Song) -> bool {
        let Some(current) = self.songs.get_mut(self.at) else {
            return false;
        };

        *current = song;
        self.touch();
        true
    }

    pub fn current_mut(&mut self) -> Option<&mut Song> {
        self.songs.get_mut(self.at)
    }

    pub fn toggle_shuffle(&mut self) {
        self.shuffle = !self.shuffle;
        self.rebuild();
    }

    pub fn set_shuffle(&mut self, on: bool, seed: Option<u64>) {
        self.shuffle = on;
        self.rebuild_with(seed);
    }

    pub fn flag_shuffle(&mut self, on: bool) {
        self.shuffle = on;
    }

    pub fn rebuild(&mut self) {
        self.rebuild_with(None);
    }

    // Shuffle permutes the songs themselves, the way Jota does, so the shared
    // queue carries the order every member plays instead of a private index map.
    fn rebuild_with(&mut self, seed: Option<u64>) {
        self.touch();
        if !self.shuffle || self.songs.is_empty() {
            return;
        }

        let anchor = self.current().map(|song| song.id.clone());
        match seed {
            Some(seed) => self.songs.shuffle(&mut seeded(seed)),
            None => self.songs.shuffle(&mut rand::rng()),
        }

        if let Some(anchor) = anchor
            && let Some(position) = self.songs.iter().position(|song| song.id == anchor)
        {
            self.at = position;
        }
    }

    // Inserts at an explicit position and keeps every copy of a song: the queue
    // is a play order, so the same track twice is two entries. A receiver must
    // end up with the sender's exact list.
    pub fn insert(&mut self, songs: Vec<Song>, at: usize) {
        if songs.is_empty() {
            return;
        }

        let empty = self.songs.is_empty();
        let at = at.min(self.songs.len());
        for (offset, song) in songs.into_iter().enumerate() {
            let index = at + offset;
            self.songs.insert(index, song);
            if !empty && index <= self.at {
                self.at += 1;
            }
        }

        self.touch();
    }

    pub fn update_video(&mut self, id: &str, video: &str) {
        if let Some(song) = self.songs.iter_mut().find(|song| song.id == id) {
            song.video_id = Some(video.to_owned());
            self.touch();
        }
    }

    pub fn next_index(&self, repeat: Repeat) -> Option<usize> {
        if self.songs.is_empty() {
            return None;
        }

        match repeat {
            Repeat::One => Some(self.at.min(self.songs.len() - 1)),
            Repeat::All => Some((self.at + 1) % self.songs.len()),
            Repeat::Off => (self.at + 1 < self.songs.len()).then_some(self.at + 1),
        }
    }

    pub fn previous_index(&self, repeat: Repeat) -> Option<usize> {
        if self.songs.is_empty() {
            return None;
        }

        match repeat {
            Repeat::One => Some(self.at.min(self.songs.len() - 1)),
            Repeat::All => Some(if self.at == 0 {
                self.songs.len() - 1
            } else {
                self.at - 1
            }),
            Repeat::Off => (self.at > 0).then(|| self.at - 1),
        }
    }

    pub fn move_song(&mut self, from: usize, to: usize) {
        if from >= self.songs.len() || to >= self.songs.len() || from == to {
            return;
        }

        let song = self.songs.remove(from);
        self.songs.insert(to, song);
        if self.at == from {
            self.at = to;
        } else if from < self.at && self.at <= to {
            self.at -= 1;
        } else if to <= self.at && self.at < from {
            self.at += 1;
        }

        self.touch();
    }

    pub fn move_after_current(&mut self, from: usize) {
        if from >= self.songs.len() || from == self.at || from == self.at + 1 {
            return;
        }

        let playing = self.songs.get(self.at).map(|song| song.id.clone());
        let song = self.songs.remove(from);
        let current = playing
            .and_then(|id| self.songs.iter().position(|song| song.id == id))
            .unwrap_or(0);
        self.songs.insert((current + 1).min(self.songs.len()), song);
        self.at = current;
        self.touch();
    }

    pub fn remove(&mut self, index: usize) {
        if index >= self.songs.len() {
            return;
        }

        self.songs.remove(index);
        if index < self.at {
            self.at -= 1;
        }

        if self.at >= self.songs.len() {
            self.at = self.songs.len().saturating_sub(1);
        }

        self.touch();
    }
}

/// An incremental queue change a room can carry as a control message, so one
/// added song or one resolved video does not rewrite every member's queue.
pub enum Edit {
    Enqueue { songs: Vec<Song>, at: usize },
    Video { id: String, video: String },
}

/// The edit between two queues, or None when the change is structural and has
/// to travel as a full snapshot. Mirrors Jota's `delta`.
pub fn delta(prev: &[Song], next: &[Song]) -> Option<Edit> {
    if next.len() > prev.len() {
        let mut start = 0;
        while start < prev.len() && prev[start].id == next[start].id {
            start += 1;
        }

        let extra = next.len() - prev.len();
        for index in start..prev.len() {
            if prev[index].id != next[index + extra].id {
                return None;
            }
        }

        return Some(Edit::Enqueue {
            songs: next[start..start + extra].to_vec(),
            at: start,
        });
    }

    if next.len() != prev.len() {
        return None;
    }

    let mut changed = None;
    for index in 0..prev.len() {
        if prev[index].id != next[index].id {
            return None;
        }

        if prev[index].video_id != next[index].video_id {
            if changed.is_some() {
                return None;
            }

            changed = Some(index);
        }
    }

    let index = changed?;
    Some(Edit::Video {
        id: next[index].id.clone(),
        video: next[index].video_id.clone()?,
    })
}

#[cfg(test)]
#[path = "queue.test.rs"]
mod tests;
