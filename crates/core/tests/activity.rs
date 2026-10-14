//! What the computer is doing, as Windows reports it (the real system).
#![cfg(windows)]

use std::os::windows::process::CommandExt;
use std::process::{Child, Command};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use sbm_core::{Activity, SystemActivity, ThrottleReason};

/// The tests load the CPU on purpose, so they take turns.
static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

/// Windows' `IDLE_PRIORITY_CLASS`: the busy loops yield to anything else
/// running, such as a game.
const IDLE_PRIORITY: u32 = 0x40;

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
                        .creation_flags(IDLE_PRIORITY)
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

/// Asks `activity` about once a second until it gives `wanted` or 10 s pass;
/// every answer, in order.
fn watch(activity: &SystemActivity, wanted: Option<ThrottleReason>) -> Vec<Option<ThrottleReason>> {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut answers = Vec::new();
    while Instant::now() < deadline && answers.last() != Some(&wanted) {
        std::thread::sleep(Duration::from_millis(1100));
        answers.push(activity.throttle_reason());
    }
    answers
}

#[test]
fn other_processes_keeping_every_core_busy_make_the_computer_busy() {
    let _turn = ONE_AT_A_TIME
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let activity = SystemActivity::default();
    activity.throttle_reason();

    let _spinners = Spinners::on_every_core();

    let answers = watch(&activity, Some(ThrottleReason::Busy));
    let busy = answers.last() == Some(&Some(ThrottleReason::Busy));
    // A full-screen app or battery power comes first, hiding a busy computer.
    let masked = [ThrottleReason::FullScreen, ThrottleReason::Battery];
    if !busy
        && answers
            .iter()
            .flatten()
            .any(|reason| masked.contains(reason))
    {
        println!("SKIPPED: busy cannot show behind {answers:?}");
        return;
    }
    assert!(busy, "{answers:?}");
}

#[test]
fn this_process_keeping_every_core_busy_does_not_make_the_computer_busy() {
    let _turn = ONE_AT_A_TIME
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let activity = SystemActivity::default();
    activity.throttle_reason();
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let cores = std::thread::available_parallelism().unwrap().get();
    let spinners: Vec<_> = (0..cores)
        .map(|_| {
            let stop = stop.clone();
            std::thread::spawn(move || {
                while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                    std::hint::spin_loop();
                }
            })
        })
        .collect();

    let mut reasons = Vec::new();
    for _ in 0..3 {
        std::thread::sleep(Duration::from_millis(1100));
        reasons.push(activity.throttle_reason());
    }
    stop.store(true, std::sync::atomic::Ordering::Relaxed);
    spinners
        .into_iter()
        .for_each(|spinner| spinner.join().unwrap());

    assert!(
        !reasons.contains(&Some(ThrottleReason::Busy)),
        "{reasons:?}"
    );
}
