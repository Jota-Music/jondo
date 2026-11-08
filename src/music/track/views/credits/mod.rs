mod state;
#[cfg(test)]
#[path = "credits.test.rs"]
mod tests;
mod view;

pub use view::{album, line};
