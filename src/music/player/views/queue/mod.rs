mod page;
mod row;

pub use page::queue;

const ROW: f32 = 64.0;
/// The title row plus the divider under it: the sheet's max height has to be
/// split between the header and the list.
const HEADER: f32 = 45.0;
/// A queue with one or two songs would otherwise shrink the panel to a sliver.
const FLOOR: f32 = 208.0;
