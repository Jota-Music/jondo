//! When cached catalog content is asked for again.
//!
//! Jota walks the whole cache every four hours and revalidates what it finds.
//! Jondo paints the cached page first and asks the source again when it is older
//! than this, so what the user looks at is never stale and a slow source never
//! holds the page: the copy on screen stays while the new one lands.

use crate::shared::kernel::app::clock::now_ms;

/// How old a cached page may be before it is asked for again.
pub const AFTER: i64 = 4 * 60 * 60 * 1000;

/// Whether an entry saved at that moment is old enough to ask again.
pub fn stale(saved: i64) -> bool {
    now_ms().saturating_sub(saved) >= AFTER
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_an_entry_past_the_interval_is_stale() {
        assert!(!stale(now_ms()));
        assert!(!stale(now_ms() - AFTER + 1000));
        assert!(stale(now_ms() - AFTER - 1000));
        assert!(stale(0), "an entry with no moment is revalidated");
    }
}
