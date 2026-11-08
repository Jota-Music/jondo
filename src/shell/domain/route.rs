use crate::shared::music::domain::SearchType;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Route {
    Home,
    Search(String),
    Playlist(String),
    Album(String),
    Artist(String),
    User(String),
    Channel(String),
    Catalog(SearchType, String),
    Following,
    Rooms,
    Settings,
}

/// A route carries its page id, so an app left open for days would otherwise
/// keep every visited page for the whole session.
const HISTORY_LIMIT: usize = 100;

pub struct History {
    items: Vec<Route>,
    at: usize,
}

impl History {
    pub(crate) fn new() -> Self {
        Self {
            items: vec![Route::Home],
            at: 0,
        }
    }

    pub fn current(&self) -> &Route {
        &self.items[self.at]
    }

    pub fn open(&mut self, route: Route) {
        if self.current() == &route {
            return;
        }

        self.items.truncate(self.at + 1);
        self.items.push(route);
        if self.items.len() > HISTORY_LIMIT {
            self.items.remove(0);
        }

        self.at = self.items.len() - 1;
    }

    pub fn back(&mut self) -> bool {
        if self.at == 0 {
            return false;
        }

        self.at -= 1;
        true
    }

    pub fn forward(&mut self) -> bool {
        if self.at + 1 >= self.items.len() {
            return false;
        }

        self.at += 1;
        true
    }

    pub fn can_back(&self) -> bool {
        self.at > 0
    }

    pub fn can_forward(&self) -> bool {
        self.at + 1 < self.items.len()
    }
}
