#[cfg(all(any(unix, windows), not(target_os = "android")))]
mod desktop;

#[cfg(all(any(unix, windows), not(target_os = "android")))]
pub use desktop::{available, cookies};

#[cfg(not(all(any(unix, windows), not(target_os = "android"))))]
mod unsupported;

#[cfg(not(all(any(unix, windows), not(target_os = "android"))))]
pub use unsupported::{available, cookies};
