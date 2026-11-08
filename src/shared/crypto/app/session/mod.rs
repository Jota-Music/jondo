use aes_gcm::{Aes256Gcm, KeyInit, Nonce, aead::Aead};
use base64::Engine;
use sha2::Sha256;

const MAGIC: &str = "JOTAENC1:";
const ITERATIONS: u32 = 100_000;

pub fn encrypt(pass: &str, plain: &str) -> Result<String, String> {
    if pass.is_empty() {
        return Ok(plain.to_owned());
    }

    let salt: [u8; 16] = rand::random();
    let nonce: [u8; 12] = rand::random();
    let mut key = [0u8; 32];
    pbkdf2::pbkdf2_hmac::<Sha256>(pass.as_bytes(), &salt, ITERATIONS, &mut key);
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|error| error.to_string())?;
    let nonce = Nonce::from(nonce);
    let sealed = cipher
        .encrypt(&nonce, plain.as_bytes())
        .map_err(|_| "cannot encrypt the session".to_owned())?;

    let mut blob = Vec::with_capacity(salt.len() + nonce.len() + sealed.len());
    blob.extend_from_slice(&salt);
    blob.extend_from_slice(&nonce);
    blob.extend_from_slice(&sealed);
    Ok(format!("{MAGIC}{}", base64(blob.as_slice())))
}

pub fn decrypt(pass: &str, data: &str) -> Result<String, String> {
    let data = data.trim();
    if !data.starts_with(MAGIC) {
        return Ok(data.to_owned());
    }

    if pass.is_empty() {
        return Err("the session needs its pass".to_owned());
    }

    let blob = unbase64(&data[MAGIC.len()..])?;
    if blob.len() < 28 {
        return Err("the session blob is damaged".to_owned());
    }

    let (salt, rest) = blob.split_at(16);
    let (nonce, sealed) = rest.split_at(12);
    let mut key = [0u8; 32];
    pbkdf2::pbkdf2_hmac::<Sha256>(pass.as_bytes(), salt, ITERATIONS, &mut key);
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|error| error.to_string())?;
    let nonce = Nonce::try_from(nonce).map_err(|_| "the session blob is damaged".to_owned())?;
    let plain = cipher
        .decrypt(&nonce, sealed)
        .map_err(|_| "the session pass does not match".to_owned())?;
    String::from_utf8(plain).map_err(|error| error.to_string())
}

pub fn sealed(data: &str) -> bool {
    data.trim().starts_with(MAGIC)
}

fn base64(data: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(data)
}

fn unbase64(text: &str) -> Result<Vec<u8>, String> {
    base64::engine::general_purpose::STANDARD
        .decode(text.trim())
        .map_err(|error| error.to_string())
}

#[cfg(test)]
#[path = "session.test.rs"]
mod tests;
