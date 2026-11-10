//! `jota.backup.json`, the document a Jota library travels in.
//!
//! Jota keeps every device converging through one JSON file in a folder the
//! user syncs with whatever they already use. The shape is Jota's: the sections
//! are read and written here exactly as it writes them, so the same folder can
//! be shared by both apps.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::shared::storage::Store;

use super::{document::*, sync::*};

pub(super) fn follows(
    document: &mut Document,
    store: &mut Store,
    mode: Mode,
    apply: bool,
) -> usize {
    let remote = document.follows.take().unwrap_or_default();

    let mut items: BTreeMap<String, i64> = store
        .follows
        .iter()
        .map(|follow| {
            let at = store
                .follows_at
                .get(&follow.id)
                .copied()
                .unwrap_or_default();
            (follow.id.clone(), at)
        })
        .collect();
    for entry in remote.items {
        let id = entry.id.trim();
        if id.is_empty() {
            continue;
        }

        let at = items.entry(id.to_owned()).or_default();
        if entry.at > *at {
            *at = entry.at;
        }
    }

    let mut removed = store.follow_graves.clone();
    for (id, at) in remote.removed {
        if at > *removed.get(&id).unwrap_or(&0) {
            removed.insert(id, at);
        }
    }

    for (id, at) in removed.clone() {
        let Some(entry) = items.get(&id).copied() else {
            continue;
        };

        if mode == Mode::Restore || entry > at {
            removed.remove(&id);
        } else {
            items.remove(&id);
        }
    }

    let mut changed = 0;
    if apply {
        let before: std::collections::HashSet<&str> = store
            .follows
            .iter()
            .map(|follow| follow.id.as_str())
            .collect();
        let mut next = Vec::with_capacity(items.len());
        for id in items.keys() {
            match store.follows.iter().find(|follow| &follow.id == id) {
                Some(known) => next.push(known.clone()),
                None => next.push(crate::shared::music::domain::Follow::canonical(id)),
            }
        }

        changed += next
            .iter()
            .filter(|follow| !before.contains(follow.id.as_str()))
            .count();
        changed += before.iter().filter(|id| !items.contains_key(**id)).count();
        store.follows = next;
        store.follows_at = items.clone();
        store.follow_graves = removed.clone();
    }

    let section = Follows {
        items: items
            .iter()
            .map(|(id, at)| FollowEntry {
                id: id.clone(),
                at: *at,
            })
            .collect(),
        removed,
    };

    document.follows = (!section.empty()).then_some(section);
    changed
}

pub(super) fn saved(document: &mut Document, store: &mut Store, mode: Mode, apply: bool) -> usize {
    let remote = document.saved.take().unwrap_or_default();

    let mut items: BTreeMap<String, SavedEntry> = store
        .saved
        .iter()
        .map(|playlist| {
            (
                playlist.id.clone(),
                SavedEntry {
                    id: playlist.id.clone(),
                    name: playlist.name.clone(),
                    cover: playlist.cover.clone().unwrap_or_default(),
                    covers: playlist.covers.clone(),
                    subtitle: playlist.subtitle.clone().unwrap_or_default(),
                    owner: playlist.owner.clone().unwrap_or_default(),
                    at: store
                        .saved_at
                        .get(&playlist.id)
                        .copied()
                        .unwrap_or_default(),
                },
            )
        })
        .collect();

    for entry in remote.items {
        let id = entry.id.trim();
        if id.is_empty() {
            continue;
        }

        match items.get_mut(id) {
            Some(known) if entry.at > known.at => *known = entry,
            Some(_) => {}
            None => {
                items.insert(id.to_owned(), entry);
            }
        }
    }

    let mut removed = store.saved_graves.clone();
    for (id, at) in remote.removed {
        if at > *removed.get(&id).unwrap_or(&0) {
            removed.insert(id, at);
        }
    }

    for (id, at) in removed.clone() {
        let Some(entry) = items.get(&id) else {
            continue;
        };

        if mode == Mode::Restore || entry.at > at {
            removed.remove(&id);
        } else {
            items.remove(&id);
        }
    }

    let mut changed = 0;
    if apply {
        let before: std::collections::HashSet<&str> = store
            .saved
            .iter()
            .map(|playlist| playlist.id.as_str())
            .collect();
        let mut next = Vec::with_capacity(items.len());
        let mut times = BTreeMap::new();
        for entry in items.values() {
            let mut playlist = store
                .saved
                .iter()
                .find(|playlist| playlist.id == entry.id)
                .cloned()
                .unwrap_or_default();
            let known = playlist.name != entry.name
                || playlist.cover.as_deref() != Some(entry.cover.as_str());
            playlist.id = entry.id.clone();
            playlist.name = entry.name.clone();
            playlist.cover = (!entry.cover.is_empty()).then(|| entry.cover.clone());
            playlist.covers = entry.covers.clone();
            playlist.subtitle = (!entry.subtitle.is_empty()).then(|| entry.subtitle.clone());
            playlist.owner = (!entry.owner.is_empty()).then(|| entry.owner.clone());
            if known {
                changed += 1;
            }

            times.insert(entry.id.clone(), entry.at);
            next.push(playlist);
        }

        changed += next
            .iter()
            .filter(|playlist| !before.contains(playlist.id.as_str()))
            .count();
        changed += before.iter().filter(|id| !items.contains_key(**id)).count();
        store.saved = next;
        store.saved_at = times;
        store.saved_graves = removed.clone();
    }

    let section = Saved {
        items: items.into_values().collect(),
        removed,
    };

    document.saved = (!section.empty()).then_some(section);
    changed
}

pub(super) fn order(document: &mut Document, store: &mut Store, mode: Mode, apply: bool) -> usize {
    let mut shelves: BTreeMap<(String, String), OrderEntry> = document
        .order
        .take()
        .unwrap_or_default()
        .into_iter()
        .map(|entry| ((entry.scope.clone(), entry.account.clone()), entry))
        .collect();

    let account = store
        .spotify
        .as_ref()
        .map(|token| token.username.clone())
        .unwrap_or_default();

    let mut changed = 0;
    let reconcile = |shelves: &mut BTreeMap<(String, String), OrderEntry>,
                     scope: &str,
                     account: &str,
                     ids: &[String],
                     at: i64,
                     changed: &mut usize| {
        let key = (scope.to_owned(), account.to_owned());
        match shelves.get(&key) {
            Some(remote) if mode == Mode::Newer && remote.at >= at => {}
            _ => {
                shelves.insert(
                    key,
                    OrderEntry {
                        scope: scope.to_owned(),
                        account: account.to_owned(),
                        ids: ids.to_vec(),
                        at,
                    },
                );
                *changed += 1;
            }
        }
    };

    if store.order_at > 0 || !store.order.is_empty() {
        reconcile(
            &mut shelves,
            "library",
            &account,
            &store.order,
            store.order_at,
            &mut changed,
        );
    }

    if store.room_at > 0 || !store.room_order.is_empty() {
        reconcile(
            &mut shelves,
            "rooms",
            "",
            &store.room_order,
            store.room_at,
            &mut changed,
        );
    }

    let mut entries: Vec<OrderEntry> = shelves.values().cloned().collect();
    entries.sort_by(|left, right| {
        left.scope
            .cmp(&right.scope)
            .then_with(|| left.account.cmp(&right.account))
    });

    if apply {
        if let Some(entry) = shelves.get(&("library".to_owned(), account.clone()))
            && (entry.at != store.order_at || entry.ids != store.order)
        {
            store.order = entry.ids.clone();
            store.order_at = entry.at;
        }

        if let Some(entry) = shelves.get(&("rooms".to_owned(), String::new()))
            && (entry.at != store.room_at || entry.ids != store.room_order)
        {
            store.room_order = entry.ids.clone();
            store.room_at = entry.at;
        }
    }

    document.order = (!entries.is_empty()).then_some(entries);
    changed
}

pub(super) fn settings(
    document: &mut Document,
    store: &mut Store,
    mode: Mode,
    apply: bool,
) -> usize {
    let mut carried = document.settings.take().unwrap_or_default();
    let mut changed = 0;

    let local = Setting {
        value: json!({"url": store.relay_url, "token": store.relay_token}),
        at: store.relay_at,
    };

    let keep_local = !store.relay_url.is_empty()
        && match carried.get("relay") {
            Some(remote) => mode == Mode::Newer && remote.at >= local.at,
            None => true,
        };

    if keep_local {
        carried.insert("relay".to_owned(), local);
    } else if let Some(remote) = carried.get("relay").cloned()
        && let (Some(url), Some(token)) = (
            remote.value.get("url").and_then(Value::as_str),
            remote.value.get("token").and_then(Value::as_str),
        )
        && (url != store.relay_url || token != store.relay_token)
        && apply
    {
        store.relay_url = url.to_owned();
        store.relay_token = token.to_owned();
        store.relay_at = remote.at;
        changed += 1;
    }

    document.settings = (!carried.is_empty()).then_some(carried);
    changed
}

/// Carries the YouTube cookies and the Spotify credentials to the other
/// devices, sealed with the pass. Nothing travels with no pass: a value that
/// cannot be opened is left in the document instead of being thrown away, so a
/// device holding the right pass can still bring it here.
pub(super) fn sessions(
    document: &mut Document,
    store: &mut Store,
    mode: Mode,
    apply: bool,
) -> usize {
    let mut carried = document.sessions.take().unwrap_or_default();
    let pass = store.session_pass.trim().to_owned();
    let apply = apply || !pass.is_empty();
    let mut changed = 0;

    if !pass.is_empty() {
        let youtube = store.youtube_cookies.trim().to_owned();
        let newer = |carried: &BTreeMap<String, Setting>, key: &str, at: i64| {
            mode == Mode::Newer && carried.get(key).is_some_and(|entry| entry.at > at)
        };

        if !youtube.is_empty()
            && !newer(&carried, "youtube", stamp_at(store.youtube_at))
            && !staged_matches(&carried, "youtube", &pass, &youtube)
            && let Ok(blob) = crate::shared::crypto::session::encrypt(&pass, &youtube)
        {
            carried.insert(
                "youtube".to_owned(),
                Setting {
                    value: serde_json::json!(blob),
                    at: stamp_at(store.youtube_at),
                },
            );
        }

        if let Some(token) = store.spotify.as_ref()
            && let Ok(plain) = serde_json::to_string(token)
            && !newer(&carried, "spotify", stamp_at(store.spotify_at))
            && !staged_matches(&carried, "spotify", &pass, &plain)
            && let Ok(blob) = crate::shared::crypto::session::encrypt(&pass, &plain)
        {
            carried.insert(
                "spotify".to_owned(),
                Setting {
                    value: serde_json::json!(blob),
                    at: stamp_at(store.spotify_at),
                },
            );
        }
    }

    if apply && !pass.is_empty() {
        if let Some(entry) = carried.get("youtube")
            && entry.at >= store.youtube_at
            && let Some(blob) = entry.value.as_str()
            && let Ok(plain) = crate::shared::crypto::session::decrypt(&pass, blob)
            && (entry.at > store.youtube_at || plain != store.youtube_cookies)
        {
            let incoming = crate::shared::youtube::full(&plain);
            if !(store.youtube_full && !incoming) {
                store.youtube_cookies = plain;
                store.youtube_full = incoming;
                store.youtube_at = entry.at;
                changed += 1;
            }
        }

        if let Some(entry) = carried.get("spotify")
            && entry.at >= store.spotify_at
            && let Some(blob) = entry.value.as_str()
            && let Ok(plain) = crate::shared::crypto::session::decrypt(&pass, blob)
            && !plain.is_empty()
            && let Ok(auth) = serde_json::from_str::<crate::shared::storage::SpotifyAuth>(&plain)
            && auth.signed_in()
            && store
                .spotify
                .as_ref()
                .is_none_or(|local| local.data != auth.data)
        {
            store.spotify = Some(auth);
            store.spotify_at = entry.at;
            changed += 1;
        }
    }

    let _ = mode;
    document.sessions = (!carried.is_empty()).then_some(carried);
    changed
}

pub(super) fn stamp_at(at: i64) -> i64 {
    if at == 0 {
        crate::shared::kernel::app::clock::now_ms()
    } else {
        at
    }
}

/// True when the merged document still carries a session sealed with a pass
/// this device does not hold, so the panel can ask for it instead of hinting at
/// nothing.
pub(super) fn sealed(document: &Document, pass: &str) -> bool {
    let pass = pass.trim();
    document.sessions.as_ref().is_some_and(|sessions| {
        sessions.values().any(|entry| {
            entry.value.as_str().is_some_and(|blob| {
                crate::shared::crypto::session::sealed(blob)
                    && (pass.is_empty()
                        || crate::shared::crypto::session::decrypt(pass, blob).is_err())
            })
        })
    })
}

pub(super) fn staged_matches(
    carried: &BTreeMap<String, Setting>,
    key: &str,
    pass: &str,
    plain: &str,
) -> bool {
    carried
        .get(key)
        .and_then(|entry| entry.value.as_str())
        .and_then(|blob| crate::shared::crypto::session::decrypt(pass, blob).ok())
        .is_some_and(|staged| staged == plain)
}
