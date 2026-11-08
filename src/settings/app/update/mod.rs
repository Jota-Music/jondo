/// The repository the About card links to. The release check only runs for a
/// build that was told where to look with `JONDO_REPO`.
pub const REPO: Option<&str> = option_env!("JONDO_REPO");
pub const PROJECT: &str = "https://github.com/Jota-Music/jondo";
pub const RELEASES: &str = "https://github.com/Jota-Music/jondo/releases";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// How this build reached the user. Only an AppImage or an APK can replace
/// itself; every other channel falls back to opening the release page.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Channel {
    AppImage,
    Android,
    External,
}

#[derive(Clone, Debug, Default)]
pub struct Info {
    pub latest: String,
    pub available: bool,
    pub url: String,
    /// Whether this build can download and swap in the release itself.
    pub installable: bool,
    /// The release asset this build installs, and its name in `SHA256SUMS`.
    pub asset: String,
    pub name: String,
    /// The release's checksum listing, empty when it carries none.
    pub sums: String,
}

fn channel() -> Channel {
    if cfg!(target_os = "android") {
        Channel::Android
    } else if cfg!(target_os = "linux") && std::env::var_os("APPIMAGE").is_some() {
        Channel::AppImage
    } else {
        Channel::External
    }
}

pub async fn check(http: &reqwest::Client) -> Result<Info, String> {
    let repo = REPO.ok_or_else(|| "no release channel".to_owned())?;
    let value: serde_json::Value = http
        .get(format!(
            "https://api.github.com/repos/{repo}/releases/latest"
        ))
        .header(reqwest::header::USER_AGENT, "jondo")
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .send()
        .await
        .map_err(|error| error.to_string())?
        .error_for_status()
        .map_err(|error| error.to_string())?
        .json()
        .await
        .map_err(|error| error.to_string())?;

    let tag = value
        .get("tag_name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let latest = tag.trim_start_matches('v').to_owned();
    let available = newer(&latest, VERSION);
    let channel = channel();
    let asset = asset(&value, channel);
    let sums = sums(&value);
    let installable = available && asset.is_some() && sums.is_some();

    Ok(Info {
        available,
        installable,
        asset: asset
            .as_ref()
            .map(|(_, url)| url.clone())
            .unwrap_or_default(),
        name: asset.map(|(name, _)| name).unwrap_or_default(),
        sums: sums.unwrap_or_default(),
        latest,
        url: value
            .get("html_url")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(RELEASES)
            .to_owned(),
    })
}

/// The asset this channel installs, as its name and download URL.
fn asset(release: &serde_json::Value, channel: Channel) -> Option<(String, String)> {
    release
        .get("assets")?
        .as_array()?
        .iter()
        .filter_map(|asset| {
            let name = asset.get("name")?.as_str()?;
            let url = asset.get("browser_download_url")?.as_str()?;
            Some((name, url))
        })
        .find(|(name, _)| matches(channel, name))
        .map(|(name, url)| (name.to_owned(), url.to_owned()))
}

fn matches(channel: Channel, name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    match channel {
        Channel::AppImage => {
            name.starts_with("jondo-")
                && name.ends_with(".appimage")
                && contains_arch(&name, std::env::consts::ARCH)
        }
        Channel::Android => name == "jondo.apk",
        Channel::External => false,
    }
}

fn contains_arch(name: &str, arch: &str) -> bool {
    if name.contains(arch) {
        return true;
    }

    match arch {
        "x86_64" => name.contains("amd64") || name.contains("x64"),
        "aarch64" => name.contains("arm64"),
        _ => false,
    }
}

fn sums(release: &serde_json::Value) -> Option<String> {
    release
        .get("assets")?
        .as_array()?
        .iter()
        .find(|asset| {
            asset
                .get("name")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|name| name.eq_ignore_ascii_case("SHA256SUMS"))
        })?
        .get("browser_download_url")?
        .as_str()
        .map(str::to_owned)
}

/// Streams a release artifact to `path`. Only an update download reaches this,
/// so the whole response is copied as it arrives instead of into memory.
pub async fn download(
    http: &reqwest::Client,
    url: &str,
    path: &std::path::Path,
) -> Result<(), String> {
    use std::io::Write as _;
    let mut response = http
        .get(url)
        .header(reqwest::header::USER_AGENT, "jondo")
        .send()
        .await
        .map_err(|error| error.to_string())?
        .error_for_status()
        .map_err(|error| error.to_string())?;
    let mut file = std::fs::File::create(path).map_err(|error| error.to_string())?;
    while let Some(chunk) = response.chunk().await.map_err(|error| error.to_string())? {
        file.write_all(&chunk).map_err(|error| error.to_string())?;
    }

    file.flush().map_err(|error| error.to_string())
}

/// Downloads, verifies and swaps in the release. The returned path is what the
/// Android installer hands to the system; the desktop relaunches on its own.
async fn install(http: &reqwest::Client, info: &Info) -> Result<std::path::PathBuf, String> {
    let dir = platform::dir()?;
    let staged = dir.join(format!(".jondo-update-{}", std::process::id()));
    let result = stage(http, info, &staged).await;
    if result.is_err() {
        let _ = std::fs::remove_file(&staged);
    }

    result
}

async fn stage(
    http: &reqwest::Client,
    info: &Info,
    staged: &std::path::Path,
) -> Result<std::path::PathBuf, String> {
    download(http, &info.asset, staged).await?;
    let listing = http
        .get(&info.sums)
        .header(reqwest::header::USER_AGENT, "jondo")
        .send()
        .await
        .map_err(|error| error.to_string())?
        .error_for_status()
        .map_err(|error| error.to_string())?
        .text()
        .await
        .map_err(|error| error.to_string())?;
    let want = parse_sums(&listing, &info.name)
        .ok_or_else(|| format!("SHA256SUMS has no entry for {}", info.name))?;
    let got = digest(staged)?;
    if !want.eq_ignore_ascii_case(&got) {
        return Err(format!("Checksum mismatch for {}", info.name));
    }

    platform::install(staged)
}

/// Reads the `<digest>  <filename>` listing, tolerating the `*` binary marker
/// and a leading path.
fn parse_sums(body: &str, name: &str) -> Option<String> {
    body.lines().find_map(|line| {
        let mut fields = line.split_whitespace();
        let digest = fields.next()?;
        let file = fields.next()?.trim_start_matches('*');
        let base = std::path::Path::new(file).file_name()?.to_str()?;
        (base == name).then(|| digest.to_owned())
    })
}

fn digest(path: &std::path::Path) -> Result<String, String> {
    use sha2::{Digest as _, Sha256};
    use std::io::Read as _;

    let mut file = std::fs::File::open(path).map_err(|error| error.to_string())?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    use std::fmt::Write as _;
    let mut hex = String::with_capacity(64);
    for byte in hasher.finalize() {
        let _ = write!(hex, "{byte:02x}");
    }

    Ok(hex)
}

fn newer(latest: &str, current: &str) -> bool {
    match (parse(latest), parse(current)) {
        (Some(latest), Some(current)) => current < latest,
        _ => false,
    }
}

/// Reads the numeric MAJOR.MINOR.PATCH prefix of a tag, tolerating a leading
/// `v` and ignoring any `-rc`/`+build` suffix.
fn parse(tag: &str) -> Option<[u64; 3]> {
    let tag = tag.trim_start_matches('v');
    let tag = tag.split(['-', '+']).next()?;
    let mut parts = tag.split('.');
    let mut out = [0u64; 3];
    for part in &mut out {
        *part = parts.next()?.parse().ok()?;
    }
    if parts.next().is_some() {
        return None;
    }

    Some(out)
}

use crate::shell::app::{Event, Jondo};

impl Jondo {
    pub fn install_update(&mut self, cx: &mut gpui::Context<Self>) {
        let Some(info) = self.settings.read(cx).update.clone() else {
            return;
        };

        if !info.installable {
            crate::shared::platform::links::open(&info.url).ok();
            return;
        }

        if self.ui.updating {
            return;
        }

        self.ui.updating = true;
        let http = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| self.http.clone());
        self.jobs
            .spawn(async move { Event::Installed(install(&http, &info).await) });
    }

    pub fn ensure_update(&mut self, cx: &mut gpui::Context<Self>) {
        if self.settings.read(cx).update_checked || REPO.is_none() {
            return;
        }

        self.settings
            .update(cx, |settings, _| settings.update_checked = true);
        let http = self.http.clone();
        self.jobs
            .spawn(async move { Event::Update(check(&http).await) });
    }
}

#[cfg(target_os = "android")]
#[path = "android.rs"]
mod platform;

#[cfg(target_os = "linux")]
#[path = "appimage.rs"]
mod platform;

#[cfg(not(any(target_os = "android", target_os = "linux")))]
#[path = "external.rs"]
mod platform;

#[cfg(test)]
#[path = "update.test.rs"]
mod tests;
