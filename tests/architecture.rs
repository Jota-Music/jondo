//! The dependency rules the tree promises, checked instead of documented.
//!
//! `shared` knows nobody. A feature never reaches into another feature except
//! where `EXPECTED` says so, and that list may only shrink. Domain types stay
//! free of the UI, the runtime and the network.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Cross-feature edges that still exist, with the phase that removes them.
/// The test only rejects *new* edges, so removing one is always allowed.
const EXPECTED: &[(&str, &str)] = &[
    // The song list, row and action menu are a shared widget living in `track`.
    // Moves to `shared/music/views` when the song widgets are lifted out.
    ("catalog", "music::track"),
    ("library", "music::track"),
    ("player", "music::track"),
    ("track", "music::library"),
    ("catalog", "music::library"),
    // Room sync translates a snapshot into queue edits. Moves to the shell
    // when rooms and player become entities.
    ("rooms", "music::player"),
    ("track", "music::player"),
    // The relay card is composed by the settings page.
    ("settings", "music::rooms"),
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn collect(dir: &Path, into: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, into);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            into.push(path);
        }
    }
}

fn relative(path: &Path) -> String {
    path.strip_prefix(root())
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Every `crate::music::<name>` a file mentions, skipping comments.
fn music_targets(path: &Path) -> BTreeSet<String> {
    let Ok(text) = fs::read_to_string(path) else {
        return BTreeSet::new();
    };

    let mut found = BTreeSet::new();
    for line in text.lines() {
        let line = line.trim_start();
        if line.starts_with("//") || line.starts_with('*') || line.starts_with("/*") {
            continue;
        }

        let mut rest = line;
        while let Some(at) = rest.find("crate::music::") {
            rest = &rest[at + "crate::music::".len()..];
            let name: String = rest
                .chars()
                .take_while(|ch| ch.is_alphanumeric() || *ch == '_')
                .collect();
            if !name.is_empty() {
                found.insert(name);
            }
        }
    }

    found
}

/// The feature a file belongs to: the folder under `music/`, or `settings`.
fn owner(rel: &str) -> Option<&str> {
    if let Some(rest) = rel.strip_prefix("music/") {
        return rest.split('/').next();
    }

    rel.starts_with("settings/").then_some("settings")
}

#[test]
fn shared_does_not_know_the_features() {
    let mut files = Vec::new();
    collect(&root().join("shared"), &mut files);

    let mut broken = Vec::new();
    for path in &files {
        let Ok(text) = fs::read_to_string(path) else {
            continue;
        };

        for (number, line) in text.lines().enumerate() {
            let line = line.trim_start();
            if line.starts_with("//") {
                continue;
            }

            for edge in ["crate::music", "crate::settings", "crate::shell"] {
                if line.contains(edge) {
                    broken.push(format!("{}:{}: {edge}", relative(path), number + 1));
                }
            }
        }
    }

    assert!(
        broken.is_empty(),
        "shared must stay below every feature:\n{}",
        broken.join("\n")
    );
}

#[test]
fn features_do_not_reach_into_other_features() {
    let mut files = Vec::new();
    collect(&root(), &mut files);

    let expected: BTreeSet<(String, String)> = EXPECTED
        .iter()
        .map(|(from, to)| (from.to_string(), to.to_string()))
        .collect();

    let mut new_edges = BTreeSet::new();
    for path in &files {
        let rel = relative(path);
        let Some(from) = owner(&rel) else {
            continue;
        };

        for target in music_targets(path) {
            if target == from {
                continue;
            }

            let edge = (from.to_string(), format!("music::{target}"));
            if !expected.contains(&edge) {
                new_edges.insert(format!("{rel}: {from} -> music::{target}"));
            }
        }
    }

    assert!(
        new_edges.is_empty(),
        "a feature may not import another feature. Compose it in the shell instead:\n{}\n\
         known edges: {EXPECTED:?}",
        new_edges.into_iter().collect::<Vec<_>>().join("\n")
    );
}

#[test]
fn domain_stays_free_of_the_ui_runtime_and_network() {
    let mut files = Vec::new();
    collect(&root(), &mut files);

    let forbidden = ["gpui", "tokio", "reqwest", "cpal", "symphonia", "rusqlite"];
    let mut broken = Vec::new();
    for path in &files {
        let rel = relative(path);
        let in_domain = rel.contains("/domain/")
            && (rel.starts_with("shared/")
                || rel.starts_with("settings/")
                || rel.starts_with("music/"));
        if !in_domain {
            continue;
        }

        let Ok(text) = fs::read_to_string(path) else {
            continue;
        };

        for (number, line) in text.lines().enumerate() {
            let line = line.trim_start();
            if line.starts_with("//") || !line.starts_with("use ") {
                continue;
            }

            for name in forbidden {
                if line.contains(name) {
                    broken.push(format!("{}:{}: {name}", rel, number + 1));
                }
            }
        }
    }

    assert!(
        broken.is_empty(),
        "domain must not depend on ui, runtime, network or storage:\n{}",
        broken.join("\n")
    );
}

#[test]
fn shell_is_the_only_composition_root() {
    let mut files = Vec::new();
    collect(&root().join("shell"), &mut files);
    assert!(
        files
            .iter()
            .any(|path| relative(path).contains("app/mod.rs")),
        "the shell should still hold the composition root"
    );
}
