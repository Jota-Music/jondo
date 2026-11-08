use std::sync::atomic::{AtomicU8, Ordering};

use crate::shared::i18n::domain::Locale;

static ACTIVE: AtomicU8 = AtomicU8::new(Locale::En as u8);

impl Locale {
    /// The save file has no choice yet, so the environment decides once.
    pub fn detect() -> Self {
        for key in ["LC_ALL", "LC_MESSAGES", "LANG"] {
            if let Ok(value) = std::env::var(key)
                && let Some(locale) = Self::parse(&value)
            {
                return locale;
            }
        }

        Self::En
    }
}

pub fn locale() -> Locale {
    match ACTIVE.load(Ordering::Relaxed) {
        1 => Locale::Es,
        _ => Locale::En,
    }
}

pub fn set(locale: Locale) {
    ACTIVE.store(locale as u8, Ordering::Relaxed);
}

/// The English text is the key, so a missing translation falls back to it
/// instead of an empty label or a dotted name.
pub fn t(key: &str) -> &str {
    match locale() {
        Locale::En => key,
        Locale::Es => spanish(key),
    }
}

/// A count with the singular key for one and the plural key for the rest.
/// The plural text carries the `{count}` placeholder.
pub fn count(n: usize, one: &str, many: &str) -> String {
    if n == 1 {
        t(one).to_owned()
    } else {
        t(many).replace("{count}", &n.to_string())
    }
}

mod es;

use es::spanish;
