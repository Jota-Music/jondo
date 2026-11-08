#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SpotifyState {
    Off,
    Connecting,
    On,
    Failed,
}
