use std::io::{Read, Write};
use std::sync::mpsc::{Sender, channel};

const CLIENT_ID: &str = "1550829418388131892";

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Activity {
    pub details: String,
    pub state: String,
    pub album: String,
    pub artwork: String,
    pub playing: bool,
    pub position: f64,
    pub duration: f64,
}

pub struct Presence {
    sender: Option<Sender<Option<Activity>>>,
}

impl Default for Presence {
    fn default() -> Self {
        Self::new()
    }
}

impl Presence {
    pub fn new() -> Self {
        Self { sender: None }
    }

    pub fn set(&mut self, activity: Activity) {
        self.send(Some(activity));
    }

    pub fn clear(&mut self) {
        self.send(None);
    }

    fn send(&mut self, activity: Option<Activity>) {
        if self.sender.is_none() {
            let (sender, receiver) = channel();
            let client = std::env::var("DISCORD_CLIENT_ID")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| CLIENT_ID.to_owned());
            std::thread::Builder::new()
                .name("jondo-discord".to_owned())
                .spawn(move || work(&client, receiver))
                .ok();
            self.sender = Some(sender);
        }

        if let Some(sender) = &self.sender {
            let _ = sender.send(activity);
        }
    }
}

#[cfg(unix)]
type Stream = std::os::unix::net::UnixStream;
#[cfg(windows)]
type Stream = std::fs::File;

#[cfg(any(unix, windows))]
fn work(client: &str, receiver: std::sync::mpsc::Receiver<Option<Activity>>) {
    let mut conn: Option<Stream> = None;
    while let Ok(activity) = receiver.recv() {
        if conn.is_none() {
            conn = connect(client);
        }

        let Some(stream) = conn.as_mut() else {
            continue;
        };

        let payload = match &activity {
            Some(activity) => serde_json::json!({
                "cmd": "SET_ACTIVITY",
                "args": {"pid": std::process::id(), "activity": wire(activity)},
                "nonce": nonce(),
            }),
            None => serde_json::json!({
                "cmd": "SET_ACTIVITY",
                "args": {"pid": std::process::id(), "activity": null},
                "nonce": nonce(),
            }),
        };

        if frame(stream, 1, payload.to_string().as_bytes()).is_err() {
            conn = None;
            continue;
        }

        drain(stream);
    }
}

#[cfg(not(any(unix, windows)))]
fn work(_client: &str, receiver: std::sync::mpsc::Receiver<Option<Activity>>) {
    while receiver.recv().is_ok() {}
}

#[cfg(unix)]
fn connect(client: &str) -> Option<Stream> {
    let mut bases = Vec::new();
    if let Ok(runtime) = std::env::var("XDG_RUNTIME_DIR") {
        bases.push(runtime);
    }

    bases.push(format!("/run/user/{}", uid()));
    bases.push(std::env::temp_dir().display().to_string());
    bases.push("/tmp".to_owned());

    for base in bases {
        for dir in [
            base.clone(),
            format!("{base}/snap.discord"),
            format!("{base}/app/com.discordapp.Discord"),
        ] {
            for index in 0..10 {
                let path = format!("{dir}/discord-ipc-{index}");
                let Ok(stream) = std::os::unix::net::UnixStream::connect(&path) else {
                    continue;
                };

                let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(3)));
                let mut stream = stream;
                let handshake = serde_json::json!({"v": 1, "client_id": client});
                if frame(&mut stream, 0, handshake.to_string().as_bytes()).is_err() {
                    continue;
                }

                let mut header = [0u8; 8];
                if stream.read_exact(&mut header).is_err() {
                    continue;
                }

                let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(120)));
                return Some(stream);
            }
        }
    }

    None
}

/// Discord serves its IPC endpoint as a byte-mode named pipe on Windows, and
/// Rust reaches it as an ordinary file.
#[cfg(windows)]
fn connect(client: &str) -> Option<Stream> {
    for index in 0..10 {
        let path = format!(r"\\.\pipe\discord-ipc-{index}");
        let Ok(stream) = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
        else {
            continue;
        };

        let handshake = serde_json::json!({"v": 1, "client_id": client});
        let mut stream = stream;
        if frame(&mut stream, 0, handshake.to_string().as_bytes()).is_err() {
            continue;
        }

        let mut header = [0u8; 8];
        if stream.read_exact(&mut header).is_err() {
            continue;
        }

        return Some(stream);
    }

    None
}

#[cfg(any(unix, windows))]
fn frame<S: Write>(stream: &mut S, op: u32, payload: &[u8]) -> std::io::Result<()> {
    let mut header = [0u8; 8];
    header[..4].copy_from_slice(&op.to_le_bytes());
    header[4..].copy_from_slice(&(payload.len() as u32).to_le_bytes());
    stream.write_all(&header)?;
    stream.write_all(payload)?;
    stream.flush()
}

/// Discord pings while the connection is open and closes on silence. Every
/// write is followed by whatever arrived in the meantime, answered with a pong.
#[cfg(any(unix, windows))]
fn drain<S: Read + Write>(stream: &mut S) {
    let mut header = [0u8; 8];
    while stream.read_exact(&mut header).is_ok() {
        let op = u32::from_le_bytes([header[0], header[1], header[2], header[3]]);
        let length = u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as usize;
        let mut payload = vec![0u8; length];
        if stream.read_exact(&mut payload).is_err() {
            return;
        }

        if op == 3 && frame(stream, 4, &payload).is_err() {
            return;
        }
    }
}

fn wire(activity: &Activity) -> serde_json::Value {
    let mut value = serde_json::json!({
        "type": 2,
        "details": activity.details,
        "state": activity.state,
    });
    if !activity.artwork.is_empty() || !activity.album.is_empty() {
        value["assets"] = serde_json::json!({
            "large_image": activity.artwork,
            "large_text": activity.album,
        });
    }

    if activity.duration > 0.0 && activity.position >= 0.0 && activity.position < activity.duration
    {
        let now = crate::shared::kernel::app::clock::now_ms();
        let mut timestamps = serde_json::json!({
            "start": now - (activity.position * 1000.0) as i64,
        });
        if activity.playing {
            timestamps["end"] =
                serde_json::json!(now + ((activity.duration - activity.position) * 1000.0) as i64);
        }

        value["timestamps"] = timestamps;
    }

    value
}

fn nonce() -> String {
    let bytes: [u8; 8] = rand::random();
    let mut text = String::with_capacity(16);
    for byte in bytes {
        use std::fmt::Write;
        let _ = write!(text, "{byte:02x}");
    }

    text
}

#[cfg(unix)]
fn uid() -> u32 {
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string("/proc/self/loginuid")
            .ok()
            .and_then(|text| text.trim().parse().ok())
            .unwrap_or(1000)
    }

    #[cfg(not(target_os = "linux"))]
    {
        0
    }
}
