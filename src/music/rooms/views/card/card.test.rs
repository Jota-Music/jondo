use super::room_menu;
use crate::music::rooms::Room;
use crate::shell::views::menu::Command;

fn names(ephemeral: bool, connected: bool) -> Vec<&'static str> {
    let room = Room {
        id: "room".to_owned(),
        ..Room::default()
    };

    room_menu(&room, ephemeral, connected)
        .iter()
        .map(|item| match &item.command {
            Command::RoomConnect(_) => "connect",
            Command::RoomLeave => "leave",
            Command::RoomSave => "save",
            Command::RoomRemove(_) => "remove",
            _ => "other",
        })
        .collect()
}

#[test]
fn the_room_menu_matches_the_shelf_actions() {
    assert_eq!(names(true, false), ["connect", "save"]);
    assert_eq!(names(true, true), ["leave", "save"]);
    assert_eq!(names(false, false), ["connect", "remove"]);
    assert_eq!(names(false, true), ["leave", "remove"]);
}
