use std::sync::atomic::AtomicBool;

mod decoder;
mod engine;
mod resample;
mod source;
mod volume;

pub use engine::Engine;

static FIRST_PACKET: AtomicBool = AtomicBool::new(false);

const BUFFER_SECONDS: f32 = 8.0;
