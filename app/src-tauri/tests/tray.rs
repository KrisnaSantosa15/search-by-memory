//! The tray: what indexing is doing, in plain words.

use jiff::Zoned;
use sbm_app::{indexing_state, next_morning};
use sbm_core::ThrottleReason;
use sbm_engine::Status;

#[test]
fn held_back_indexing_says_why() {
    let held = |reason| Status {
        pending: 10,
        throttled: Some(reason),
        ..Status::default()
    };

    assert_eq!(
        indexing_state(&held(ThrottleReason::FullScreen), &Zoned::now()),
        "Indexing waits while a full-screen app is open"
    );
    assert_eq!(
        indexing_state(&held(ThrottleReason::Battery), &Zoned::now()),
        "Indexing waits until the computer is plugged in"
    );
    assert_eq!(
        indexing_state(&held(ThrottleReason::Busy), &Zoned::now()),
        "Indexing waits while the computer is busy"
    );
}

#[test]
fn running_indexing_says_how_many_files_are_left_or_that_all_are_done() {
    let pending = |pending| Status {
        pending,
        ..Status::default()
    };

    assert_eq!(
        indexing_state(&pending(1), &Zoned::now()),
        "Indexing: 1 file to go"
    );
    assert_eq!(
        indexing_state(&pending(12_345), &Zoned::now()),
        "Indexing: 12,345 files to go"
    );
    assert_eq!(
        indexing_state(&pending(0), &Zoned::now()),
        "All files are indexed"
    );
}

#[test]
fn paused_indexing_says_so_whatever_else_holds_it_back() {
    let paused = Status {
        pending: 10,
        paused: true,
        throttled: Some(ThrottleReason::Battery),
        ..Status::default()
    };

    assert_eq!(indexing_state(&paused, &Zoned::now()), "Indexing is paused");
}

/// `time` in Jakarta, e.g. "2026-10-15T22:00".
fn jakarta(time: &str) -> Zoned {
    format!("{time}[Asia/Jakarta]").parse().unwrap()
}

#[test]
fn pause_until_tomorrow_ends_at_the_next_six_in_the_morning() {
    assert_eq!(
        next_morning(&jakarta("2026-10-15T22:00")),
        jakarta("2026-10-16T06:00")
    );
    assert_eq!(
        next_morning(&jakarta("2026-10-16T01:00")),
        jakarta("2026-10-16T06:00")
    );
    assert_eq!(
        next_morning(&jakarta("2026-10-16T06:00")),
        jakarta("2026-10-17T06:00")
    );
}
