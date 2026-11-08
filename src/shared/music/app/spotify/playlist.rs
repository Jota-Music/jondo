use std::sync::Arc;

use librespot_protocol::playlist4_external::SelectedListContent;
use protobuf::Message;

use crate::shared::music::domain::{Playlist, Source};

use super::*;
use super::{session::*, tracks::*};

pub async fn playlist(
    client: &reqwest::Client,
    session: &Arc<Session>,
    id: &str,
) -> Result<Playlist, String> {
    let bytes = get_bytes(client, session, &format!("/playlist/v2/playlist/{id}")).await?;
    let content = SelectedListContent::parse_from_bytes(&bytes)
        .map_err(|error| format!("spotify playlist: {error}"))?;

    let attributes = content.attributes.get_or_default();
    let name = attributes.name.clone().unwrap_or_default();
    let cover = attributes.picture.as_deref().and_then(picture);
    let owner = content.owner_username.as_deref().and_then(non_empty);

    let uris: Vec<String> = content
        .contents
        .into_option()
        .map(|contents| {
            contents
                .items
                .iter()
                .map(|item| item.uri().to_owned())
                .filter(|uri| uri.starts_with("spotify:track:"))
                .collect()
        })
        .unwrap_or_default();

    let songs = tracks(client, session, &uris).await;
    let cover = cover.or_else(|| songs.iter().find_map(|song| song.cover.clone()));
    let covers = songs
        .iter()
        .filter_map(|song| song.cover.clone())
        .take(4)
        .collect();
    let length = content.length.unwrap_or(0).max(0) as u32;
    let track_count = if length > 0 {
        length
    } else {
        songs.len() as u32
    };

    Ok(Playlist {
        id: format!("spotify:playlist:{id}"),
        name,
        subtitle: owner.clone(),
        cover,
        covers,
        owner,
        source: Source::Spotify,
        songs,
        track_count,
        ..Default::default()
    })
}

pub(super) async fn metadata(
    client: &reqwest::Client,
    session: &Arc<Session>,
    kind: &str,
    id: &str,
) -> Result<Vec<u8>, String> {
    let gid = id_to_hex(id);
    let response = authorized(session, "spotify metadata", |access| {
        client
            .get(format!("{SPCLIENT}/metadata/4/{kind}/{gid}"))
            .header(reqwest::header::AUTHORIZATION, format!("Bearer {access}"))
            .header(reqwest::header::ACCEPT, "application/octet-stream")
    })
    .await?;
    let status = response.status();
    let bytes = response
        .bytes()
        .await
        .map_err(|error| format!("spotify metadata: {error}"))?;
    if !status.is_success() {
        return Err(format!("spotify metadata answered {status}"));
    }

    if bytes.is_empty() {
        return Err("spotify metadata: empty response".to_owned());
    }

    Ok(bytes.to_vec())
}

pub(crate) fn id_to_hex(id: &str) -> String {
    if id.len() == 32 && id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return id.to_owned();
    }

    const ALPHABET: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let mut value: u128 = 0;
    for byte in id.bytes() {
        let Some(digit) = ALPHABET.iter().position(|entry| *entry == byte) else {
            return id.to_owned();
        };

        value = value.saturating_mul(62).saturating_add(digit as u128);
    }

    format!("{value:032x}")
}

pub(crate) fn hex_to_base62(hex: &str) -> String {
    let Ok(value) = u128::from_str_radix(hex, 16) else {
        return hex.to_owned();
    };

    const ALPHABET: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let mut encoded = Vec::new();
    let mut rest = value;
    if rest == 0 {
        encoded.push(b'0');
    } else {
        while rest > 0 {
            encoded.push(ALPHABET[(rest % 62) as usize]);
            rest /= 62;
        }
    }

    encoded.reverse();
    let text = String::from_utf8(encoded).unwrap_or_default();
    format!("{text:0>22}")
}
