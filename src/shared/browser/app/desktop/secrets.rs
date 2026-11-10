//! The key Chromium encrypts cookie values with, and how a value gets opened.
//!
//! Linux keeps the password in the desktop keyring and derives an AES-128-CBC
//! key from it with one PBKDF2 iteration; macOS asks Keychain for the same kind
//! of password and derives it with 1003. Windows does not use a password at all:
//! `Local State` carries a 32-byte key wrapped by DPAPI, and the cookies are
//! AES-256-GCM under it. Firefox is in the clear on all three.

use super::jars::Jar;

/// Opens the encrypted values of one jar. The secrets are read once, so a jar
/// with a hundred matching rows does not hit the keyring a hundred times.
#[cfg(not(windows))]
pub(super) struct Decryptor {
    secrets: Vec<String>,
}

#[cfg(not(windows))]
impl Decryptor {
    pub(super) fn new(jar: &Jar) -> Self {
        let secrets = if jar.firefox {
            Vec::new()
        } else {
            secrets(jar.name)
        };

        Self { secrets }
    }

    #[cfg(test)]
    pub(super) fn for_test(secret: &str) -> Self {
        Self {
            secrets: vec![secret.to_owned()],
        }
    }

    /// `v10`/`v11`, then AES-128-CBC with the key the keyring password derives
    /// (PBKDF2-SHA1, `saltysalt`). The IV is sixteen spaces, which is why the
    /// scheme is only as strong as the password.
    pub(super) fn value(&self, encrypted: &[u8], version: i64) -> Option<String> {
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

        for secret in &self.secrets {
            if let Some(mut plain) = cbc(secret, body, iterations()) {
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
}

/// macOS derives with more work than Linux for the same scheme.
#[cfg(target_os = "macos")]
fn iterations() -> u32 {
    1003
}

#[cfg(all(unix, not(target_os = "macos")))]
fn iterations() -> u32 {
    1
}

#[cfg(not(windows))]
pub(super) fn cbc(secret: &str, body: &[u8], rounds: u32) -> Option<Vec<u8>> {
    use aes::Aes128;
    use aes::cipher::{Array, BlockCipherDecrypt, KeyInit};

    let mut key = [0u8; 16];
    pbkdf2::pbkdf2_hmac::<sha1::Sha1>(secret.as_bytes(), b"saltysalt", rounds, &mut key);
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

/// Linux: the desktop keyring, then the passwords Chromium falls back to when no
/// keyring is available.
#[cfg(all(unix, not(target_os = "macos")))]
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

/// macOS: Keychain holds one generic password per browser, named after it.
#[cfg(target_os = "macos")]
pub(super) fn secrets(browser: &str) -> Vec<String> {
    let label = keyring_label(browser);
    let Ok(output) = std::process::Command::new("security")
        .args(["find-generic-password", "-s", &label, "-w"])
        .output()
    else {
        return Vec::new();
    };

    let secret = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if secret.is_empty() {
        Vec::new()
    } else {
        vec![secret]
    }
}

#[cfg(not(windows))]
pub(super) fn keyring_label(browser: &str) -> String {
    if browser.eq_ignore_ascii_case("edge") {
        "Microsoft Edge Safe Storage".to_owned()
    } else {
        format!("{browser} Safe Storage")
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
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

#[cfg(all(unix, not(target_os = "macos")))]
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

/// Windows: `Local State` carries the AES-256-GCM key wrapped by DPAPI, so the
/// jar's own folder is the secret. Read once per profile.
#[cfg(windows)]
pub(super) struct Decryptor {
    key: Option<[u8; 32]>,
}

#[cfg(windows)]
impl Decryptor {
    pub(super) fn new(jar: &Jar) -> Self {
        let key = if jar.firefox {
            None
        } else {
            local_state_key(&jar.path)
        };

        Self { key }
    }

    /// `v10` followed by AES-256-GCM: a 12-byte nonce, the ciphertext and the
    /// tag. Chrome 127 and later can seal some values with an app-bound key
    /// (`v20`) that another process cannot unwrap, so those rows stay closed.
    pub(super) fn value(&self, encrypted: &[u8], _version: i64) -> Option<String> {
        use aes_gcm::aead::Aead as _;
        use aes_gcm::{Aes256Gcm, KeyInit as _, Nonce};

        if encrypted.len() < 3 + 12 + 16 || &encrypted[..3] != b"v10" {
            return None;
        }

        let body = &encrypted[3..];
        let nonce: [u8; 12] = body.get(..12)?.try_into().ok()?;
        let cipher = Aes256Gcm::new_from_slice(self.key.as_ref()?).ok()?;
        let plain = cipher.decrypt(&Nonce::from(nonce), &body[12..]).ok()?;
        String::from_utf8(plain).ok()
    }
}

#[cfg(windows)]
fn local_state_key(cookies: &std::path::Path) -> Option<[u8; 32]> {
    use base64::Engine as _;

    // <User Data>/<profile>/Network/Cookies -> <User Data>/Local State
    let state = cookies.parent()?.parent()?.parent()?.join("Local State");
    let text = std::fs::read_to_string(state).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    let encoded = value.get("os_crypt")?.get("encrypted_key")?.as_str()?;
    let raw = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .ok()?;
    let wrapped = raw.strip_prefix(b"DPAPI")?;
    let key = unprotect(wrapped)?;
    key.try_into().ok()
}

#[cfg(windows)]
fn unprotect(data: &[u8]) -> Option<Vec<u8>> {
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{CRYPT_INTEGER_BLOB, CryptUnprotectData};

    let mut input = CRYPT_INTEGER_BLOB {
        cbData: data.len() as u32,
        pbData: data.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };

    let ok = unsafe {
        CryptUnprotectData(
            &mut input,
            std::ptr::null_mut(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            &mut output,
        )
    };

    if ok == 0 || output.pbData.is_null() {
        return None;
    }

    let plain =
        unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec() };
    unsafe { LocalFree(output.pbData as *mut _) };
    Some(plain)
}
