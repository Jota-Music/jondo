mod bar;
mod page;
mod search;

pub use page::show;
pub(crate) use search::placeholder;

const FIELD_HEIGHT: f32 = 40.0;
const MOBILE_FIELD_HEIGHT: f32 = 44.0;
const GAP: f32 = 8.0;
const MOBILE_GAP: f32 = 6.0;
const MOBILE_PADDING: f32 = 16.0;
