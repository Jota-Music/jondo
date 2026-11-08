use serde::{Deserialize, Serialize};

use super::Source;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FollowKind {
    #[default]
    Artist,
    User,
    Channel,
}

/// One follow, stored under the key Jota writes into its backup: `artist:<id>`,
/// `youtube:<channel>` and a bare handle for a Spotify user.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Follow {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub kind: FollowKind,
    #[serde(default)]
    pub source: Option<Source>,
}

impl Follow {
    pub fn key(kind: FollowKind, id: &str) -> String {
        match kind {
            FollowKind::Artist => format!("artist:{id}"),
            FollowKind::Channel => format!("youtube:{id}"),
            FollowKind::User => id.to_owned(),
        }
    }

    /// The id the source service understands, without the follow key prefix.
    pub fn target(&self) -> &str {
        self.id
            .strip_prefix("artist:")
            .or_else(|| self.id.strip_prefix("youtube:"))
            .or_else(|| self.id.strip_prefix("user:"))
            .unwrap_or(&self.id)
    }

    pub fn spotify(kind: FollowKind, id: &str, name: &str, image: Option<String>) -> Self {
        Self {
            id: Self::key(kind, id),
            name: name.to_owned(),
            image,
            kind,
            source: Some(Source::Spotify),
        }
    }

    pub fn youtube(id: &str, name: &str, image: Option<String>) -> Self {
        Self {
            id: Self::key(FollowKind::Channel, id),
            name: name.to_owned(),
            image,
            kind: FollowKind::Channel,
            source: Some(Source::YouTube),
        }
    }

    /// Reads a key from either app's store or backup and normalizes it to one
    /// of the three forms this app routes on.
    pub fn canonical(value: &str) -> Self {
        let value = value.trim();
        if value.is_empty() {
            return Self::default();
        }

        if let Some(id) = value
            .strip_prefix("artist:")
            .or_else(|| value.strip_prefix("spotify:artist:"))
        {
            return Self::spotify(FollowKind::Artist, id, id, None);
        }

        if let Some(id) = value.strip_prefix("youtube:") {
            return Self::youtube(id, id, None);
        }

        if let Some(id) = value
            .strip_prefix("user:")
            .or_else(|| value.strip_prefix("spotify:user:"))
        {
            return Self::spotify(FollowKind::User, id, id, None);
        }

        // A bare base62 id is an artist Jondo wrote before follows carried a
        // prefix; Jota only stores users bare.
        if value.len() == 22 && value.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Self::spotify(FollowKind::Artist, value, value, None);
        }

        Self::spotify(FollowKind::User, value, value, None)
    }
}

#[cfg(test)]
#[path = "follow.test.rs"]
mod tests;
