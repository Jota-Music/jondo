#[cfg(target_os = "android")]
pub mod android;
pub mod desktop;
pub mod links;
pub mod memory;
pub mod pin;
#[cfg(target_os = "android")]
pub mod saf;
