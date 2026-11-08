#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    Order,
    SearchType,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Drag {
    Disc,
    Seek,
    Mini,
    Volume,
    Controls,
    Preview,
    PreviewVolume,
}

/// The list a reorder drag lifted a row out of, kept so the frame clock can
/// keep scrolling it while the pointer rests at an edge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reorder {
    Queue,
    Playlist(String),
}
