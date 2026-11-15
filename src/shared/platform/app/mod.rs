#[cfg(target_os = "android")]
pub mod android;
pub mod desktop;
pub mod links;
pub mod memory;
pub mod pin;
#[cfg(unix)]
pub mod relaunch;
#[cfg(target_os = "android")]
pub mod saf;
