use super::*;
use crate::shared::music::domain::Song;

fn song(id: &str) -> Song {
    Song {
        id: id.to_owned(),
        ..Default::default()
    }
}

#[test]
fn next_honours_repeat_and_order() {
    let mut queue = Queue::default();
    queue.set(vec![song("a"), song("b"), song("c")], 0);

    assert_eq!(queue.next_index(Repeat::Off), Some(1));
    queue.at = 2;
    assert_eq!(queue.next_index(Repeat::Off), None);
    assert_eq!(queue.next_index(Repeat::All), Some(0));
    assert_eq!(queue.next_index(Repeat::One), Some(2));
}

#[test]
fn previous_honours_repeat_and_order() {
    let mut queue = Queue::default();
    queue.set(vec![song("a"), song("b"), song("c")], 0);

    assert_eq!(queue.previous_index(Repeat::Off), None);
    assert_eq!(queue.previous_index(Repeat::All), Some(2));
    assert_eq!(queue.previous_index(Repeat::One), Some(0));
    queue.at = 1;
    assert_eq!(queue.previous_index(Repeat::Off), Some(0));
}

#[test]
fn moving_keeps_the_current_track_selected() {
    let mut queue = Queue::default();
    queue.set(vec![song("a"), song("b"), song("c"), song("d")], 2);

    queue.move_song(0, 3);
    let ids: Vec<&str> = queue.songs.iter().map(|song| song.id.as_str()).collect();
    assert_eq!(ids, ["b", "c", "d", "a"]);
    assert_eq!(queue.at, 1);
    assert_eq!(queue.current().map(|song| song.id.as_str()), Some("c"));

    queue.move_song(1, 0);
    assert_eq!(queue.at, 0);
    assert_eq!(queue.current().map(|song| song.id.as_str()), Some("c"));
}

#[test]
fn removing_keeps_the_current_track_valid() {
    let mut queue = Queue::default();
    queue.set(vec![song("a"), song("b"), song("c")], 2);
    queue.remove(0);
    assert_eq!(queue.current().map(|song| song.id.as_str()), Some("c"));
    assert_eq!(queue.at, 1);
}

#[test]
fn queue_edits_travel_incrementally() {
    let prev = vec![song("a"), song("b")];
    let added = vec![song("a"), song("x"), song("b")];
    match delta(&prev, &added) {
        Some(Edit::Enqueue { songs, at }) => {
            assert_eq!(at, 1);
            assert_eq!(songs[0].id, "x");
        }
        _ => panic!("an insert is an enqueue"),
    }

    let mut linked = prev.clone();
    linked[1].video_id = Some("video".to_owned());
    match delta(&prev, &linked) {
        Some(Edit::Video { id, video }) => {
            assert_eq!(id, "b");
            assert_eq!(video, "video");
        }
        _ => panic!("a resolved video is a youtube edit"),
    }

    assert!(delta(&prev, &[song("b"), song("a")]).is_none());
    assert!(delta(&prev, &[song("a")]).is_none());
}

#[test]
fn seeded_shuffle_matches_on_every_member() {
    let songs = (0..12)
        .map(|index| song(&format!("song-{index}")))
        .collect::<Vec<_>>();
    let mut first = Queue::default();
    let mut second = Queue::default();
    first.set(songs.clone(), 4);
    second.set(songs, 4);

    first.set_shuffle(true, Some(42));
    second.set_shuffle(true, Some(42));
    let ids = |queue: &Queue| {
        queue
            .songs
            .iter()
            .map(|song| song.id.clone())
            .collect::<Vec<_>>()
    };

    assert_eq!(ids(&first), ids(&second));
    assert_eq!(first.current().map(|song| song.id.as_str()), Some("song-4"));
}

#[test]
fn every_queue_change_moves_the_revision() {
    let mut queue = Queue::default();
    let start = queue.revision;
    queue.set(vec![song("a"), song("b"), song("c")], 0);
    let set = queue.revision;
    assert_ne!(set, start, "a new queue is a new revision");

    queue.insert(vec![song("x")], 1);
    let inserted = queue.revision;
    assert_ne!(inserted, set, "an insert is a new revision");

    queue.move_song(0, 2);
    let moved = queue.revision;
    assert_ne!(moved, inserted, "a move is a new revision");

    queue.remove(0);
    assert_ne!(queue.revision, moved, "a removal is a new revision");
}

#[test]
fn inserting_keeps_the_announced_position() {
    let mut queue = Queue::default();
    queue.set(vec![song("a"), song("b"), song("c")], 0);
    queue.insert(vec![song("x")], 1);
    let ids = queue
        .songs
        .iter()
        .map(|song| song.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ids, ["a", "x", "b", "c"]);
}

#[test]
fn inserting_keeps_duplicates_and_handles_an_empty_queue() {
    let mut queue = Queue::default();
    queue.insert(vec![song("a")], 0);
    assert_eq!(queue.current().map(|song| song.id.as_str()), Some("a"));
    assert_eq!(queue.at, 0);

    queue.insert(vec![song("b")], 1);
    queue.set(vec![song("a"), song("b")], 0);
    queue.insert(vec![song("a")], 2);
    let ids = queue
        .songs
        .iter()
        .map(|song| song.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ids, ["a", "b", "a"]);
    assert_eq!(queue.current().map(|song| song.id.as_str()), Some("a"));
    assert!(queue.same(&[song("a"), song("a"), song("b")]));
    assert!(!queue.same(&[song("a"), song("b")]));
}

#[test]
fn replacing_now_keeps_the_queue() {
    let mut queue = Queue::default();
    queue.set(vec![song("a"), song("b"), song("c")], 1);
    let before = queue.revision;
    assert!(queue.replace_current(song("x")));
    let ids = queue
        .songs
        .iter()
        .map(|song| song.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ids, ["a", "x", "c"]);
    assert_eq!(queue.at, 1);
    assert_ne!(queue.revision, before);
    assert!(!Queue::default().replace_current(song("x")));
}
