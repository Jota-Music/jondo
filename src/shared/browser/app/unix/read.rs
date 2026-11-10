//! Reads the YouTube session a desktop browser already holds.
//!
//! Chromium keeps its cookie values AES-128-CBC encrypted under a password the
//! desktop keyring holds; Firefox keeps them in the clear. Both jars are copied
//! before reading, because the browser keeps the live file locked and what is
//! still in the write-ahead log would otherwise be missed. Android cannot reach
//! a browser jar: the phone version accepts pasted cookies instead.

use std::path::Path;

use super::{jars::*, secrets::*};

pub(super) fn header(jar: &Jar) -> Result<String, String> {
    let (rows, version) = read(jar)?;
    let secrets = if jar.firefox {
        Vec::new()
    } else {
        secrets(jar.name)
    };

    let mut order = Vec::new();
    let mut picked: Vec<(String, String, u8)> = Vec::new();
    let mut locked = false;
    for row in rows {
        let value = if !row.value.is_empty() {
            row.value
        } else if !row.encrypted.is_empty() {
            match decrypt(&row.encrypted, version, &secrets) {
                Some(value) => value,
                None => {
                    locked = true;
                    continue;
                }
            }
        } else {
            continue;
        };

        let rank = rank(&row.host);
        if rank == 0 {
            continue;
        }

        match picked.iter_mut().find(|(name, _, _)| *name == row.name) {
            Some(entry) if rank > entry.2 => {
                entry.1 = value;
                entry.2 = rank;
            }
            Some(_) => {}
            None => {
                order.push(row.name.clone());
                picked.push((row.name, value, rank));
            }
        }
    }

    if picked.is_empty() && locked {
        return Err("the browser cookies could not be decrypted: unlock the keyring".to_owned());
    }

    let header = order
        .into_iter()
        .filter_map(|name| {
            picked
                .iter()
                .find(|(candidate, _, _)| *candidate == name)
                .map(|(_, value, _)| format!("{name}={value}"))
        })
        .collect::<Vec<_>>()
        .join("; ");
    Ok(header)
}

pub(super) fn rank(host: &str) -> u8 {
    if host.contains("youtube.com") {
        2
    } else if host.contains("google.com") {
        1
    } else {
        0
    }
}

pub(super) struct Row {
    pub(super) host: String,
    pub(super) name: String,
    pub(super) value: String,
    pub(super) encrypted: Vec<u8>,
}

pub(super) fn read(jar: &Jar) -> Result<(Vec<Row>, i64), String> {
    let dir = std::env::temp_dir().join(format!(
        "jondo-cookies-{}-{}",
        std::process::id(),
        crate::shared::kernel::app::clock::now_ms()
    ));
    std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    let name = jar
        .path
        .file_name()
        .map(|name| name.to_owned())
        .ok_or_else(|| "no cookie jar".to_owned())?;
    let target = dir.join(&name);
    copy(&jar.path, &target)?;
    let _ = copy(
        Path::new(&format!("{}-wal", jar.path.display())),
        Path::new(&format!("{}-wal", target.display())),
    );
    let _ = copy(
        Path::new(&format!("{}-shm", jar.path.display())),
        Path::new(&format!("{}-shm", target.display())),
    );

    let rows = query(&target, jar.firefox);
    let _ = std::fs::remove_dir_all(&dir);
    rows
}

pub(super) fn copy(from: &Path, to: &Path) -> Result<(), String> {
    std::fs::copy(from, to)
        .map(|_| ())
        .map_err(|error| format!("{}: {error}", from.display()))
}

pub(super) fn query(path: &Path, firefox: bool) -> Result<(Vec<Row>, i64), String> {
    use rusqlite::OpenFlags;
    let conn = rusqlite::Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|error| error.to_string())?;

    let firefox = firefox || has_firefox_table(&conn);
    if firefox {
        let mut statement = conn
            .prepare("SELECT host, name, value FROM moz_cookies WHERE host LIKE '%youtube.com%' OR host LIKE '%google.com%'")
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([], |row| {
                Ok(Row {
                    host: row.get(0)?,
                    name: row.get(1)?,
                    value: row.get(2)?,
                    encrypted: Vec::new(),
                })
            })
            .map_err(|error| error.to_string())?;
        let rows = rows
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| error.to_string())?;
        return Ok((rows, 0));
    }

    let version = conn
        .query_row("SELECT value FROM meta WHERE key = 'version'", [], |row| {
            row.get::<_, String>(0)
        })
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);

    let mut statement = conn
        .prepare("SELECT host_key, name, value, encrypted_value FROM cookies WHERE host_key LIKE '%youtube.com%' OR host_key LIKE '%google.com%'")
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok(Row {
                host: row.get(0)?,
                name: row.get(1)?,
                value: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                encrypted: row.get(3)?,
            })
        })
        .map_err(|error| error.to_string())?;
    let rows = rows
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())?;
    Ok((rows, version))
}

pub(super) fn has_firefox_table(conn: &rusqlite::Connection) -> bool {
    conn.query_row(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'moz_cookies'",
        [],
        |row| row.get::<_, String>(0),
    )
    .is_ok()
}
