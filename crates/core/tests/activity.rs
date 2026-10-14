//! What the computer is doing, as Windows reports it (the real system).
#![cfg(windows)]

use std::process::{Child, Command};
use std::time::{Duration, Instant};

use sbm_core::{Activity, SystemActivity, ThrottleReason};

/// Busy loops in other processes, one per core; killed when dropped.
struct Spinners(Vec<Child>);

impl Spinners {
    fn on_every_core() -> Self {
        let cores = std::thread::available_parallelism().unwrap().get();
        Self(
            (0..cores)
                .map(|_| {
                    Command::new("cmd")
                        .args(["/c", "for /l %i in (0,0,1) do @rem"])
                        .spawn()
                        .unwrap()
                })
                .collect(),
        )
    }
}

impl Drop for Spinners {
    fn drop(&mut self) {
        for child in &mut self.0 {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// Asks `activity` about once a second until `until` holds or 10 s pass.
fn watch(activity: &SystemActivity, until: impl Fn(Option<ThrottleReason>) -> bool) -> bool {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(1100));
        if until(activity.throttle_reason()) {
            return true;
        }
    }
    false
}

#[test]
fn other_processes_keeping_every_core_busy_make_the_computer_busy() {
    let activity = SystemActivity::default();
    activity.throttle_reason();

    let _spinners = Spinners::on_every_core();

    assert!(watch(&activity, |reason| reason == Some(ThrottleReason::Busy)));
}
