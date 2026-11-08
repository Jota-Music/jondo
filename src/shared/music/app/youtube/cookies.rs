use std::collections::HashSet;

/// Whether the cookies can open the account's own lists, which is what the
/// settings panel calls a full session. `SAPISID` alone signs requests but
/// leaves private playlists closed.
pub fn full(cookies: &str) -> bool {
    cookies.split(';').any(|pair| {
        pair.trim()
            .split_once('=')
            .is_some_and(|(name, _)| matches!(name, "__Secure-3PSID" | "__Secure-1PSID" | "SSID"))
    })
}

/// Whether two headers name the same Google session, ignoring the cookies
/// YouTube rotates. A browser read can then refresh a stored snapshot without
/// changing accounts.
pub fn same(a: &str, b: &str) -> bool {
    sapisid(a).is_some() && sapisid(a) == sapisid(b)
}

/// Turns whatever the user pasted into a `Cookie` header: a cookies.txt export
/// (tab separated), a `Cookie:` header, or the pairs themselves. Nothing comes
/// back when there is no `SAPISID`, since that is what signs the request.
pub fn parse_cookies(text: &str) -> Option<String> {
    let mut pairs: Vec<(String, String)> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.contains('\t') {
            let fields: Vec<&str> = line.split('\t').collect();
            if fields.len() >= 7 {
                pairs.push((fields[5].trim().to_owned(), fields[6].trim().to_owned()));
            }

            continue;
        }

        let line = line.strip_prefix("Cookie:").unwrap_or(line).trim();
        for part in line.split(';') {
            if let Some((name, value)) = part.split_once('=') {
                pairs.push((name.trim().to_owned(), value.trim().to_owned()));
            }
        }
    }

    let mut out = String::new();
    let mut seen = HashSet::new();
    for (name, value) in pairs {
        if name.is_empty() || value.is_empty() || !seen.insert(name.clone()) {
            continue;
        }

        if !out.is_empty() {
            out.push_str("; ");
        }

        out.push_str(&name);
        out.push('=');
        out.push_str(&value);
    }

    (sapisid(&out).is_some()).then_some(out)
}

pub(super) fn sapisid(cookies: &str) -> Option<&str> {
    cookies
        .split(';')
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(name, _)| matches!(*name, "SAPISID" | "__Secure-3PAPISID"))
        .map(|(_, value)| value)
}

pub(super) fn sapisid_hash(cookies: &str, origin: &str) -> Option<String> {
    use sha1::{Digest, Sha1};
    let secret = sapisid(cookies)?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or_default();
    let mut hasher = Sha1::new();
    hasher.update(format!("{stamp} {secret} {origin}"));
    let digest = hasher.finalize();
    let mut hash = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write;
        let _ = write!(hash, "{byte:02x}");
    }

    Some(format!("SAPISIDHASH {stamp}_{hash}"))
}
