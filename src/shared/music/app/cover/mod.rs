mod cache;
mod load;
mod spin;
#[cfg(test)]
#[path = "cover.test.rs"]
mod tests;

pub use cache::Covers;

const SIZES: [u32; 2] = [160, 320];
const LIMITS: [usize; 2] = [64, 16];
const THUMB: usize = 0;
const FULL: usize = 1;
const FAILED_LIMIT: usize = 512;
const COVER_SIZE: u32 = 320;
const SPIN_SIZE: u32 = 160;
const SPIN_STEP: f32 = 0.005;
