use std::sync::mpsc;
use std::time::Duration;

use super::*;

#[test]
fn a_stopped_pool_closes_without_waiting_for_blocked_work() {
    let mut jobs = Jobs::new();
    let (release, blocked) = mpsc::channel::<()>();
    jobs.blocking(move || {
        let _ = blocked.recv();
        Event::Browsers(Vec::new())
    });
    std::thread::sleep(Duration::from_millis(50));

    let (done, closed) = mpsc::channel();
    std::thread::spawn(move || {
        jobs.stop();
        done.send(jobs).ok();
    });

    let Ok(jobs) = closed.recv_timeout(Duration::from_secs(2)) else {
        panic!("closing waited for a blocking task that never returned");
    };

    jobs.run(async {});
    jobs.task(|| {});
    jobs.spawn(async { Event::Browsers(Vec::new()) });
    jobs.blocking(|| Event::Browsers(Vec::new()));
    jobs.emit()(Event::Browsers(Vec::new()));
    drop(release);
}
