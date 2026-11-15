use std::collections::VecDeque;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

const LIMIT: usize = 4000;

/// Every other crate in the tree (GPU probe, Wayland plumbing, audio backends)
/// logs per frame or per call, which buries this app's own lines.
const APP: &str = "jondo";

/// Set when `RUST_LOG` names a level: the operator asked for every crate.
static LOUD: AtomicBool = AtomicBool::new(false);
static LINES: Mutex<VecDeque<String>> = Mutex::new(VecDeque::new());
static SINK: OnceLock<Sink> = OnceLock::new();

/// Android already has a logger, so this one carries it: the platform copy keeps
/// logcat working and the ring below is what the logs panel and the share sheet
/// read. Installing only the platform logger left the phone with no log to give.
#[cfg(target_os = "android")]
struct Sink {
    android: android_logger::AndroidLogger,
}

#[cfg(not(target_os = "android"))]
struct Sink;

fn sink() -> Sink {
    #[cfg(target_os = "android")]
    {
        let config = android_logger::Config::default()
            .with_max_level(log::max_level())
            .with_tag(APP);
        Sink {
            android: android_logger::AndroidLogger::new(config),
        }
    }

    #[cfg(not(target_os = "android"))]
    Sink
}

fn wanted(level: log::Level, target: &str) -> bool {
    LOUD.load(Ordering::Relaxed) || target.starts_with(APP) || level <= log::LevelFilter::Warn
}

impl log::Log for Sink {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::max_level()
            && (cfg!(target_os = "android") || wanted(metadata.level(), metadata.target()))
    }

    fn log(&self, record: &log::Record) {
        #[cfg(target_os = "android")]
        self.android.log(record);

        if !wanted(record.level(), record.target()) {
            return;
        }

        if record.level() == log::Level::Warn && missed_system_font(&record.args().to_string()) {
            return;
        }

        let line = format!(
            "{} [{}] {}",
            crate::shared::kernel::app::clock::now_ms(),
            record.level(),
            record.args()
        );
        #[cfg(not(target_os = "android"))]
        eprintln!("{line}");
        push(line);
    }

    fn flush(&self) {}
}

/// Installs the logger. Without `RUST_LOG` this app speaks at `info` and every
/// other crate only at `warn`; setting `RUST_LOG` (e.g. `RUST_LOG=debug`) lifts
/// that filter and applies the level to everyone.
pub fn install() {
    let asked = std::env::var("RUST_LOG")
        .ok()
        .and_then(|value| value.parse::<log::LevelFilter>().ok());
    LOUD.store(asked.is_some(), Ordering::Relaxed);
    log::set_max_level(asked.unwrap_or(log::LevelFilter::Info));
    let _ = log::set_logger(SINK.get_or_init(sink));

    // The ring above dies with the process, and a release build aborts on the
    // spot, so a panic that lands after the window is gone leaves nothing to
    // read. This one line survives it.
    let stderr = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        crash(&describe(info));
        stderr(info);
    }));
}

fn describe(info: &std::panic::PanicHookInfo<'_>) -> String {
    let payload = info.payload();
    let message = payload
        .downcast_ref::<&str>()
        .map(|text| (*text).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown panic".to_owned());
    let place = info
        .location()
        .map(|at| format!("{}:{}:{}", at.file(), at.line(), at.column()))
        .unwrap_or_default();

    format!(
        "thread '{}' at {place}: {message}",
        std::thread::current().name().unwrap_or("unnamed")
    )
}

/// Appends a panic to the file the user can hand over, next to the state. A
/// crash that cannot be read costs the whole report.
fn crash(line: &str) {
    let Some(dir) = crate::shared::storage::dir() else {
        return;
    };

    let _ = std::fs::create_dir_all(&dir);
    let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("crash.log"))
    else {
        return;
    };

    let _ = writeln!(
        file,
        "{} [PANIC] {line}",
        crate::shared::kernel::app::clock::now_ms()
    );
    let _ = file.flush();
}

fn push(line: String) {
    if let Ok(mut lines) = LINES.lock() {
        if lines.len() == LIMIT {
            lines.pop_front();
        }

        lines.push_back(line);
    }
}

/// GPUI maps the virtual `.SystemUIFont` to a platform name ("IBM Plex Sans" on
/// Linux) before walking its fallback stack. A desktop without that family
/// warns once before drawing in the fallback it already picked; the app names
/// Inter on the theme, so the miss costs nothing and would only repeat the same
/// line on every start.
fn missed_system_font(message: &str) -> bool {
    message.contains("failed to load font") && message.contains(".SystemUIFont")
}

pub fn text() -> String {
    match LINES.lock() {
        Ok(lines) => lines.iter().cloned().collect::<Vec<_>>().join("\n"),
        Err(_) => String::new(),
    }
}

/// Writes the log where the user can find it without a native file dialog: the
/// Downloads folder when the system reports one, the app data folder otherwise.
#[cfg(not(target_os = "android"))]
pub fn save() -> Result<PathBuf, String> {
    let dir = directories::UserDirs::new()
        .and_then(|dirs| dirs.download_dir().map(|path| path.to_path_buf()))
        .or_else(crate::shared::storage::dir)
        .ok_or_else(|| "no folder to save the logs".to_owned())?;

    let path = dir.join("jondo.log");
    let mut file = std::fs::File::create(&path).map_err(|error| error.to_string())?;
    file.write_all(text().as_bytes())
        .map_err(|error| error.to_string())?;
    Ok(path)
}

/// Android has no save dialog that answers a filesystem path, so the log is
/// staged inside the app's files dir and handed to the share chooser through
/// the FileProvider.
#[cfg(target_os = "android")]
pub fn save() -> Result<PathBuf, String> {
    let dir = crate::shared::platform::android::data_dir()
        .ok_or_else(|| "no folder to save the logs".to_owned())?
        .join("logs");
    std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;

    let path = dir.join("jondo.log");
    let mut file = std::fs::File::create(&path).map_err(|error| error.to_string())?;
    file.write_all(text().as_bytes())
        .map_err(|error| error.to_string())?;
    crate::shared::platform::android::share::send(&path, "text/plain", "jondo.log")?;
    Ok(path)
}

#[cfg(test)]
#[path = "logs.test.rs"]
mod tests;
