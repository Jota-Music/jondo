// The application builds on the Kit facade alone; the alias keeps the existing
// `gpui::` paths and macro output resolving through it.
extern crate gpui_kit as gpui;

pub mod music;
pub mod settings;
pub mod shared;
pub mod shell;
