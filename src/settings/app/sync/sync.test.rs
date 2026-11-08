use crate::settings::app::backup::Stats;
use crate::shared::storage::Store;

use super::accept_backup;

fn changed() -> Stats {
    Stats {
        playlists: 1,
        ..Default::default()
    }
}

#[test]
fn a_pass_from_before_the_last_save_is_discarded() {
    let result = Ok(changed());
    assert!(accept_backup(8, 7, Store::default(), &result).is_none());
}

#[test]
fn a_fresh_pass_replaces_the_store_and_reports_changes() {
    let result = Ok(changed());
    let (store, changed) = accept_backup(7, 7, Store::default(), &result).expect("fresh pass");
    assert!(changed);
    assert!(store.playlists.is_empty());
}

#[test]
fn a_failed_pass_still_applies_its_status() {
    let result = Err("no folder".to_owned());
    let (store, changed) = accept_backup(7, 7, Store::default(), &result).expect("fresh pass");
    assert!(!changed);
    assert!(store.playlists.is_empty());
}
