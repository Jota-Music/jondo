//! Reads the YouTube session a desktop browser already holds.
//!
//! Chromium keeps its cookie values AES-128-CBC encrypted under a password the
//! desktop keyring holds; Firefox keeps them in the clear. Both jars are copied
//! before reading, because the browser keeps the live file locked and what is
//! still in the write-ahead log would otherwise be missed. Android cannot reach
//! a browser jar: the phone version accepts pasted cookies instead.

use super::{read::*, secrets::*};

#[test]
fn domain_rank_prefers_youtube() {
    assert_eq!(rank(".youtube.com"), 2);
    assert_eq!(rank(".google.com"), 1);
    assert_eq!(rank("example.com"), 0);
}

#[test]
fn chromium_values_decrypt_with_the_fallback_password() {
    let secret = "peanuts";
    let mut key = [0u8; 16];
    pbkdf2::pbkdf2_hmac::<sha1::Sha1>(secret.as_bytes(), b"saltysalt", 1, &mut key);
    use aes::cipher::{Array, BlockCipherEncrypt, KeyInit};
    let cipher = aes::Aes128::new(&Array::from(key));
    let plain = b"session-value";
    let mut padded = plain.to_vec();
    let pad = 16 - plain.len() % 16;
    padded.extend(std::iter::repeat_n(pad as u8, pad));

    let mut previous = [b' '; 16];
    let mut body = Vec::new();
    for chunk in padded.chunks(16) {
        let block: [u8; 16] = chunk.try_into().unwrap();
        let mut buffer = Array::from(block);
        for (index, byte) in buffer.iter_mut().enumerate() {
            *byte ^= previous[index];
        }

        cipher.encrypt_block(&mut buffer);
        body.extend_from_slice(buffer.as_slice());
        previous = buffer.as_slice().try_into().unwrap();
    }

    let mut value_bytes = b"v10".to_vec();
    value_bytes.extend_from_slice(&body);
    let value = decrypt(&value_bytes, 0, &["peanuts".to_owned()]).unwrap();
    assert_eq!(value, "session-value");
}
