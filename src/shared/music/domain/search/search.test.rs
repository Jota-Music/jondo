use super::*;

fn songs() -> Vec<Song> {
    vec![
        Song {
            name: "Beta".into(),
            artists: vec!["Two".into()],
            duration: 200,
            ..Default::default()
        },
        Song {
            name: "Alpha".into(),
            artists: vec!["One".into()],
            duration: 100,
            ..Default::default()
        },
    ]
}

#[test]
fn the_query_matches_names_and_artists() {
    let all = songs();
    assert_eq!(visible(&all, "alpha", Order::Added, true), vec![1]);
    assert_eq!(visible(&all, "two", Order::Added, true), vec![0]);
    assert_eq!(
        visible(&all, "zzz", Order::Added, true),
        Vec::<usize>::new()
    );
}

#[test]
fn ordering_reverses_and_sorts_by_duration() {
    let all = songs();
    assert_eq!(visible(&all, "", Order::Reverse, true), vec![1, 0]);
    assert_eq!(visible(&all, "", Order::DurationAsc, true), vec![1, 0]);
    assert_eq!(visible(&all, "", Order::DurationDesc, true), vec![0, 1]);
}

#[test]
fn a_plain_list_keeps_every_index_in_source_order() {
    let all = songs();
    assert_eq!(visible(&all, "alpha", Order::Reverse, false), vec![0, 1]);
}

#[test]
fn only_an_unmodified_local_list_reorders() {
    let mut all = songs();
    all[0].id = "beta".to_owned();
    all[1].id = "alpha".to_owned();
    let ids = vec!["beta".to_owned(), "alpha".to_owned()];

    assert!(reorderable(true, &all, &ids));
    assert!(!reorderable(false, &all, &ids));
    assert!(!reorderable(
        true,
        &all,
        &["alpha".to_owned(), "beta".to_owned()]
    ));
    assert!(!reorderable(true, &all[..1], &ids[..1]));
}
