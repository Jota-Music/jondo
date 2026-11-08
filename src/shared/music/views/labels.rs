use crate::shared::i18n::t;
use crate::shared::music::domain::{Order, SearchType};

impl Order {
    pub fn label(self) -> &'static str {
        match self {
            Self::Added => t("Oldest"),
            Self::Reverse => t("Newest"),
            Self::DurationAsc => t("Shortest"),
            Self::DurationDesc => t("Longest"),
        }
    }
}

impl SearchType {
    pub fn label(self) -> &'static str {
        match self {
            Self::Track => t("Track"),
            Self::User => t("User"),
            Self::Album => t("Album"),
            Self::Playlist => t("Playlist"),
            Self::Artist => t("Artist"),
        }
    }
}
