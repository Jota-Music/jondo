//! Reads the YouTube session a desktop browser already holds.
//!
//! Chromium keeps its cookie values AES-128-CBC encrypted under a password the
//! desktop keyring holds; Firefox keeps them in the clear. Both jars are copied
//! before reading, because the browser keeps the live file locked and what is
//! still in the write-ahead log would otherwise be missed. Android cannot reach
//! a browser jar: the phone version accepts pasted cookies instead.

use std::path::PathBuf;

use super::read::*;

pub(super) struct Root {
    name: &'static str,
    path: &'static str,
    home: bool,
    firefox: bool,
}

const ROOTS: &[Root] = &[
    Root {
        name: "Brave",
        path: "BraveSoftware/Brave-Browser",
        home: false,
        firefox: false,
    },
    Root {
        name: "Brave",
        path: ".var/app/com.brave.Browser/config/BraveSoftware/Brave-Browser",
        home: true,
        firefox: false,
    },
    Root {
        name: "Chrome",
        path: "google-chrome",
        home: false,
        firefox: false,
    },
    Root {
        name: "Chrome",
        path: ".var/app/com.google.Chrome/config/google-chrome",
        home: true,
        firefox: false,
    },
    Root {
        name: "Chromium",
        path: "chromium",
        home: false,
        firefox: false,
    },
    Root {
        name: "Chromium",
        path: ".var/app/org.chromium.Chromium/config/chromium",
        home: true,
        firefox: false,
    },
    Root {
        name: "Edge",
        path: "microsoft-edge",
        home: false,
        firefox: false,
    },
    Root {
        name: "Edge",
        path: ".var/app/com.microsoft.Edge/config/microsoft-edge",
        home: true,
        firefox: false,
    },
    Root {
        name: "Vivaldi",
        path: "vivaldi",
        home: false,
        firefox: false,
    },
    Root {
        name: "Vivaldi",
        path: ".var/app/com.vivaldi.Vivaldi/config/vivaldi",
        home: true,
        firefox: false,
    },
    Root {
        name: "Firefox",
        path: ".mozilla/firefox",
        home: true,
        firefox: true,
    },
    Root {
        name: "Firefox",
        path: "snap/firefox/common/.mozilla/firefox",
        home: true,
        firefox: true,
    },
    Root {
        name: "Firefox",
        path: ".var/app/org.mozilla.firefox/.mozilla/firefox",
        home: true,
        firefox: true,
    },
    Root {
        name: "Zen",
        path: ".zen",
        home: true,
        firefox: true,
    },
    Root {
        name: "Zen",
        path: ".var/app/app.zen_browser.zen/.zen",
        home: true,
        firefox: true,
    },
    Root {
        name: "LibreWolf",
        path: ".librewolf",
        home: true,
        firefox: true,
    },
    Root {
        name: "LibreWolf",
        path: ".var/app/io.gitlab.librewolf-community/.librewolf",
        home: true,
        firefox: true,
    },
    Root {
        name: "Waterfox",
        path: ".waterfox",
        home: true,
        firefox: true,
    },
    Root {
        name: "Waterfox",
        path: ".var/app/net.waterfox.waterfox/.waterfox",
        home: true,
        firefox: true,
    },
    Root {
        name: "Floorp",
        path: ".floorp",
        home: true,
        firefox: true,
    },
    Root {
        name: "Floorp",
        path: ".var/app/one.ablaze.floorp/.floorp",
        home: true,
        firefox: true,
    },
];

pub(super) struct Jar {
    pub(super) name: &'static str,
    pub(super) path: PathBuf,
    pub(super) firefox: bool,
}

/// Every browser on this machine whose jar holds something that can sign a
/// YouTube request, named once each.
pub fn available() -> Vec<&'static str> {
    let mut names: Vec<&'static str> = Vec::new();
    for jar in jars() {
        if names.contains(&jar.name) {
            continue;
        }

        if read(&jar).is_ok_and(|(rows, _)| rows.iter().any(|row| session(&row.name))) {
            names.push(jar.name);
        }
    }

    names
}

/// The `Cookie` header for youtube.com from the first profile of that browser
/// that holds a session. A partial jar (one that only signs requests) is kept
/// as a fallback, since it is still more than nothing.
pub fn cookies(name: &str) -> Result<String, String> {
    let mut partial = None;
    let mut last = None;
    let mut found = false;
    for jar in jars()
        .into_iter()
        .filter(|jar| jar.name.eq_ignore_ascii_case(name))
    {
        found = true;
        match header(&jar) {
            Ok(header) if header.is_empty() => {}
            Ok(header) => {
                if crate::shared::youtube::full(&header) {
                    return Ok(header);
                }

                if partial.is_none() {
                    partial = Some(header);
                }
            }
            Err(error) => last = Some(error),
        }
    }

    if let Some(header) = partial {
        return Ok(header);
    }

    if !found {
        return Err(format!("{name} has no cookie jar"));
    }

    Err(last.unwrap_or_else(|| format!("{name} is not signed in to YouTube")))
}

pub(super) fn jars() -> Vec<Jar> {
    let home = directories::UserDirs::new()
        .map(|dirs| dirs.home_dir().to_path_buf())
        .unwrap_or_default();
    let config = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".config"));

    let mut list = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for root in ROOTS {
        let base = if root.home { &home } else { &config };
        let dir = base.join(root.path);
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };

        for entry in entries.flatten() {
            let profile = entry.file_name().to_string_lossy().into_owned();
            if !entry.path().is_dir() || profile.starts_with('.') || special(&profile) {
                continue;
            }

            let path = if root.firefox {
                entry.path().join("cookies.sqlite")
            } else {
                let network = entry.path().join("Network").join("Cookies");
                if network.is_file() {
                    network
                } else {
                    entry.path().join("Cookies")
                }
            };

            if !path.is_file() || !seen.insert(path.clone()) {
                continue;
            }

            list.push(Jar {
                name: root.name,
                path,
                firefox: root.firefox,
            });
        }
    }

    list
}

pub(super) fn special(profile: &str) -> bool {
    matches!(
        profile,
        "Guest Profile" | "System Profile" | "Crash Reports"
    )
}

pub(super) fn session(name: &str) -> bool {
    matches!(name, "__Secure-3PSID" | "__Secure-1PSID" | "SSID")
}
