use std::rc::Rc;

use crate::shared::music::domain::{Order, Song, visible};

/// The rows a list page shows: indices into the song set, the songs those
/// indices point at, and the ids of the visible ones. A page keeps the table
/// while its key holds, so a frame never copies the list it paints.
#[derive(Default, Clone)]
pub struct Table {
    pub(crate) key: Option<String>,
    pub rows: Rc<Vec<usize>>,
    pub full: Rc<Vec<Song>>,
    pub ids: Rc<Vec<String>>,
}

impl Table {
    pub fn build(songs: Vec<Song>, query: &str, order: Order, searchable: bool) -> Self {
        let rows = visible(&songs, query, order, searchable);
        let ids = rows
            .iter()
            .filter_map(|index| songs.get(*index).map(|song| song.id.clone()))
            .collect();
        Self {
            key: None,
            rows: Rc::new(rows),
            full: Rc::new(songs),
            ids: Rc::new(ids),
        }
    }
}

/// A list a page derives from the library, held while its key does. Sharing the
/// `Rc` keeps a frame from rebuilding the list it paints.
pub struct Cache<T> {
    pub(crate) key: Option<String>,
    pub value: Rc<Vec<T>>,
}

impl<T> Default for Cache<T> {
    fn default() -> Self {
        Self {
            key: None,
            value: Rc::new(Vec::new()),
        }
    }
}

impl<T> Clone for Cache<T> {
    fn clone(&self) -> Self {
        Self {
            key: self.key.clone(),
            value: self.value.clone(),
        }
    }
}
