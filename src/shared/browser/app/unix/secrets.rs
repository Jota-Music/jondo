//! Reads the YouTube session a desktop browser already holds.
//!
//! Chromium keeps its cookie values AES-128-CBC encrypted under a password the
//! desktop keyring holds; Firefox keeps them in the clear. Both jars are copied
//! before reading, because the browser keeps the live file locked and what is
//! still in the write-ahead log would otherwise be missed. Android cannot reach
//! a browser jar: the phone version accepts pasted cookies instead.

pub(super) fn secrets(browser: &str) -> Vec<String> {
    let mut secrets = libsecret(browser);
    if secrets.is_empty()
        && let Some(secret) = kwallet(browser)
    {
        secrets.push(secret);
    }

    secrets.push("peanuts".to_owned());
    secrets.push(String::new());
    secrets
}

pub(super) fn keyring_label(browser: &str) -> String {
    if browser.eq_ignore_ascii_case("edge") {
        "Microsoft Edge Safe Storage".to_owned()
    } else {
        format!("{browser} Safe Storage")
    }
}

pub(super) fn libsecret(browser: &str) -> Vec<String> {
    let Ok(output) = std::process::Command::new("secret-tool")
        .args([
            "search",
            "--all",
            "xdg:schema",
            "chrome_libsecret_os_crypt_password_v2",
        ])
        .output()
    else {
        return Vec::new();
    };

    let text = String::from_utf8_lossy(&output.stdout);
    let label = keyring_label(browser);
    let want = browser.to_lowercase();
    let mut own = Vec::new();
    let mut other = Vec::new();
    for block in text.split("\n[") {
        let mut fields = std::collections::HashMap::new();
        for line in block.lines() {
            if let Some((key, value)) = line.trim().split_once(" = ") {
                fields.insert(key.trim().to_owned(), value.trim().to_owned());
            }
        }

        if !fields
            .get("label")
            .is_some_and(|value| value.eq_ignore_ascii_case(&label))
        {
            continue;
        }

        let Some(secret) = fields.get("secret").filter(|secret| !secret.is_empty()) else {
            continue;
        };

        if fields
            .get("attribute.application")
            .is_some_and(|value| value.to_lowercase().contains(&want))
        {
            own.push(secret.clone());
        } else {
            other.push(secret.clone());
        }
    }

    own.extend(other);
    own
}

pub(super) fn kwallet(browser: &str) -> Option<String> {
    let label = keyring_label(browser);
    let product = label.strip_suffix(" Safe Storage").unwrap_or(browser);
    for entry in [label.clone(), "Chrome Safe Storage".to_owned()] {
        let Ok(output) = std::process::Command::new("kwallet-query")
            .args(["-f", &format!("{product} Keys"), "-r", &entry, "kwallet"])
            .output()
        else {
            continue;
        };

        let secret = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        if !secret.is_empty() {
            return Some(secret);
        }
    }

    None
}

/// Chromium: `v10`/`v11`, then AES-128-CBC with the key the keyring password
/// derives (PBKDF2-SHA1, one iteration, `saltysalt`). The IV is sixteen spaces,
/// which is why the scheme is only as strong as the password.
pub(super) fn decrypt(encrypted: &[u8], version: i64, secrets: &[String]) -> Option<String> {
    if encrypted.len() <= 3 {
        return None;
    }

    let scheme = &encrypted[..3];
    if scheme != b"v10" && scheme != b"v11" {
        return None;
    }

    let body = &encrypted[3..];
    if body.is_empty() || !body.len().is_multiple_of(16) {
        return None;
    }

    for secret in secrets {
        if let Some(mut plain) = cbc(secret, body) {
            if version >= 24 {
                if plain.len() <= 32 {
                    continue;
                }

                plain.drain(..32);
            }

            return String::from_utf8(plain).ok();
        }
    }

    None
}

pub(super) fn cbc(secret: &str, body: &[u8]) -> Option<Vec<u8>> {
    use aes::Aes128;
    use aes::cipher::{Array, BlockCipherDecrypt, KeyInit};

    let mut key = [0u8; 16];
    pbkdf2::pbkdf2_hmac::<sha1::Sha1>(secret.as_bytes(), b"saltysalt", 1, &mut key);
    let cipher = Aes128::new(&Array::from(key));

    let mut previous = [b' '; 16];
    let mut plain = Vec::with_capacity(body.len());
    for chunk in body.chunks(16) {
        let block: [u8; 16] = chunk.try_into().ok()?;
        let mut buffer = Array::from(block);
        cipher.decrypt_block(&mut buffer);
        for (index, byte) in buffer.iter().enumerate() {
            plain.push(byte ^ previous[index]);
        }

        previous = block;
    }

    let pad = *plain.last()? as usize;
    if pad == 0 || pad > 16 || pad > plain.len() {
        return None;
    }

    if plain[plain.len() - pad..]
        .iter()
        .any(|byte| *byte as usize != pad)
    {
        return None;
    }

    plain.truncate(plain.len() - pad);
    Some(plain)
}
