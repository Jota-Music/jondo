use crate::shared::i18n::domain::Locale;
use crate::shared::music::domain::Room;
use crate::shared::music::domain::{Order, SearchType};
use crate::shared::music::domain::{Playlist, Song};
use crate::shell::domain::Route;

#[derive(Clone)]
pub enum Command {
    Play(Vec<Song>, usize),
    Toggle,
    Enqueue(Vec<Song>),
    Pick(Vec<Song>),
    Select(Vec<Song>),
    Clear,
    SelectOne(String, usize),
    ToggleOne(String, usize),
    Remove(String, Vec<String>),
    Edit(Song),
    Copy(String),
    QueuePlay(usize),
    QueueDown(usize),
    QueueSnap(usize),
    QueueRemove(usize),
    PlaylistPlay(String),
    AlbumPlay(String),
    PlaylistSave(Playlist),
    Follow(crate::shared::music::domain::Follow),
    PlaylistRename(String, String),
    PlaylistDelete(String),
    RoomConnect(Room),
    RoomLeave,
    RoomSave,
    RoomRemove(String),
    Open(Route),
    SearchType(SearchType),
    PlaylistOrder(Order),
    ChannelOrder(Order),
    Locale(Locale),
    Browser(String),
    Interval(u64),
}
