#[cfg(all(unix, not(target_os = "android")))]
mod unix;

#[cfg(all(unix, not(target_os = "android")))]
pub use unix::{available, cookies};

#[cfg(not(all(unix, not(target_os = "android"))))]
mod unsupported;

#[cfg(not(all(unix, not(target_os = "android"))))]
pub use unsupported::{available, cookies};
