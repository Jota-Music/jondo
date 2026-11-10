//! Reads the YouTube session a desktop browser already holds.
//!
//! Chromium keeps its cookie values encrypted — under a keyring password on
//! Linux and macOS, under DPAPI on Windows — while Firefox keeps them in the
//! clear. Both jars are copied before reading, because the browser keeps the
//! live file locked and what is still in the write-ahead log would otherwise be
//! missed. Android cannot reach a browser jar: the phone version accepts pasted
//! cookies instead.

use std::path::PathBuf;

use super::read::*;

/// Where a browser keeps its profiles. The same browser uses a different folder
/// on each desktop, and on Windows even a different root per browser.
pub(super) enum Base {
    /// The per-user configuration: `~/.config`, `~/Library/Application Support`
    /// or `%APPDATA%`.
    Config,
    /// The per-user local data: `%LOCALAPPDATA%` on Windows, the configuration
    /// folder everywhere else. Only the Windows roots use it.
    #[cfg_attr(not(windows), allow(dead_code))]
    Local,
    /// Straight from the home directory, where the Linux and macOS browsers keep
    /// their profiles.
    #[cfg_attr(windows, allow(dead_code))]
    Home,
}

pub(super) struct Root {
    pub(super) name: &'static str,
    pub(super) path: &'static str,
    pub(super) base: Base,
    pub(super) firefox: bool,
}

#[cfg(target_os = "linux")]
pub(super) const ROOTS: &[Root] = &[
    Root {
        name: "Brave",
        path: "BraveSoftware/Brave-Browser",
        base: Base::Config,
        firefox: false,
    },
    Root {
        name: "Brave",
        path: ".var/app/com.brave.Browser/config/BraveSoftware/Brave-Browser",
        base: Base::Home,
        firefox: false,
    },
    Root {
        name: "Chrome",
        path: "google-chrome",
        base: Base::Config,
        firefox: false,
    },
    Root {
        name: "Chrome",
        path: ".var/app/com.google.Chrome/config/google-chrome",
        base: Base::Home,
        firefox: false,
    },
    Root {
        name: "Chromium",
        path: "chromium",
        base: Base::Config,
        firefox: false,
    },
    Root {
        name: "Chromium",
        path: ".var/app/org.chromium.Chromium/config/chromium",
        base: Base::Home,
        firefox: false,
    },
    Root {
        name: "Edge",
        path: "microsoft-edge",
        base: Base::Config,
        firefox: false,
    },
    Root {
        name: "Edge",
        path: ".var/app/com.microsoft.Edge/config/microsoft-edge",
        base: Base::Home,
        firefox: false,
    },
    Root {
        name: "Vivaldi",
        path: "vivaldi",
        base: Base::Config,
        firefox: false,
    },
    Root {
        name: "Vivaldi",
        path: ".var/app/com.vivaldi.Vivaldi/config/vivaldi",
        base: Base::Home,
        firefox: false,
    },
    Root {
        name: "Firefox",
        path: ".mozilla/firefox",
        base: Base::Home,
        firefox: true,
    },
    Root {
        name: "Firefox",
        path: "snap/firefox/common/.mozilla/firefox",
        base: Base::Home,
        firefox: true,
    },
    Root {
        name: "Firefox",
        path: ".var/app/org.mozilla.firefox/.mozilla/firefox",
        base: Base::Home,
        firefox: true,
    },
    Root {
        name: "Zen",
        path: ".zen",
        base: Base::Home,
        firefox: true,
    },
    Root {
        name: "Zen",
        path: ".var/app/app.zen_browser.zen/.zen",
        base: Base::Home,
        firefox: true,
    },
    Root {
        name: "LibreWolf",
        path: ".librewolf",
        base: Base::Home,
        firefox: true,
    },
    Root {
        name: "LibreWolf",
        path: ".var/app/io.gitlab.librewolf-community/.librewolf",
        base: Base::Home,
        firefox: true,
    },
    Root {
        name: "Waterfox",
        path: ".waterfox",
        base: Base::Home,
        firefox: true,
    },
    Root {
        name: "Waterfox",
        path: ".var/app/net.waterfox.waterfox/.waterfox",
        base: Base::Home,
        firefox: true,
    },
    Root {
        name: "Floorp",
        path: ".floorp",
        base: Base::Home,
        firefox: true,
    },
    Root {
        name: "Floorp",
        path: ".var/app/one.ablaze.floorp/.floorp",
        base: Base::Home,
        firefox: true,
    },
];

#[cfg(target_os = "macos")]
pub(super) const ROOTS: &[Root] = &[
    Root {
        name: "Brave",
        path: "BraveSoftware/Brave-Browser",
        base: Base::Config,
        firefox: false,
    },
    Root {
        name: "Chrome",
        path: "Google/Chrome",
        base: Base::Config,
        firefox: false,
    },
    Root {
        name: "Chromium",
        path: "Chromium",
        base: Base::Config,
        firefox: false,
    },
    Root {
        name: "Edge",
        path: "Microsoft Edge",
        base: Base::Config,
        firefox: false,
    },
    Root {
        name: "Vivaldi",
        path: "Vivaldi",
        base: Base::Config,
        firefox: false,
    },
    Root {
        name: "Firefox",
        path: "Library/Application Support/Firefox/Profiles",
        base: Base::Home,
        firefox: true,
    },
    Root {
        name: "Zen",
        path: "Library/Application Support/zen/Profiles",
        base: Base::Home,
        firefox: true,
    },
    Root {
        name: "LibreWolf",
        path: "Library/Application Support/librewolf/Profiles",
        base: Base::Home,
        firefox: true,
    },
    Root {
        name: "Waterfox",
        path: "Library/Application Support/Waterfox/Profiles",
        base: Base::Home,
        firefox: true,
    },
    Root {
        name: "Floorp",
        path: "Library/Application Support/Floorp/Profiles",
        base: Base::Home,
        firefox: true,
    },
];

#[cfg(windows)]
pub(super) const ROOTS: &[Root] = &[
    Root {
        name: "Brave",
        path: "BraveSoftware/Brave-Browser/User Data",
        base: Base::Local,
        firefox: false,
    },
    Root {
        name: "Chrome",
        path: "Google/Chrome/User Data",
        base: Base::Local,
        firefox: false,
    },
    Root {
        name: "Chromium",
        path: "Chromium/User Data",
        base: Base::Local,
        firefox: false,
    },
    Root {
        name: "Edge",
        path: "Microsoft/Edge/User Data",
        base: Base::Local,
        firefox: false,
    },
    Root {
        name: "Vivaldi",
        path: "Vivaldi/User Data",
        base: Base::Local,
        firefox: false,
    },
    Root {
        name: "Firefox",
        path: "Mozilla/Firefox/Profiles",
        base: Base::Config,
        firefox: true,
    },
    Root {
        name: "Zen",
        path: "zen/Profiles",
        base: Base::Config,
        firefox: true,
    },
    Root {
        name: "LibreWolf",
        path: "librewolf/Profiles",
        base: Base::Config,
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

    let mut list = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for root in ROOTS {
        let base = match root.base {
            Base::Home => home.clone(),
            Base::Config => config(&home),
            Base::Local => local(&home),
        };

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

#[cfg(target_os = "linux")]
fn config(home: &std::path::Path) -> PathBuf {
    std::env::var("XDG_CONFIG_HOME")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".config"))
}

#[cfg(target_os = "linux")]
fn local(home: &std::path::Path) -> PathBuf {
    std::env::var("XDG_DATA_HOME")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".local/share"))
}

#[cfg(target_os = "macos")]
fn config(home: &std::path::Path) -> PathBuf {
    home.join("Library/Application Support")
}

#[cfg(target_os = "macos")]
fn local(home: &std::path::Path) -> PathBuf {
    config(home)
}

#[cfg(windows)]
fn config(home: &std::path::Path) -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join("AppData/Roaming"))
}

#[cfg(windows)]
fn local(home: &std::path::Path) -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join("AppData/Local"))
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
