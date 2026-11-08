use std::sync::{Mutex, MutexGuard};

/// A decoder or network thread panics on a dead stream, not while it tears the
/// value apart: the data behind a poisoned lock is still usable, so take it
/// instead of turning one thread's panic into the caller's.
pub fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
