mod cache;
mod disk;
mod spin;
#[cfg(test)]
#[path = "cover.test.rs"]
mod tests;

pub use cache::Covers;
pub use disk::path;

const SIZES: [u32; 2] = [160, 320];
const LIMITS: [usize; 2] = [128, 32];
const THUMB: usize = 0;
const FULL: usize = 1;
const DISK_LIMIT: usize = 800;
const FAILED_LIMIT: usize = 2048;
const COVER_SIZE: u32 = 320;
const SPIN_SIZE: u32 = 160;
const SPIN_STEP: f32 = 0.005;
