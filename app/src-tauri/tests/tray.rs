//! The tray: what indexing is doing, in plain words.

use sbm_app::indexing_state;
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
        indexing_state(&held(ThrottleReason::FullScreen)),
        "Indexing waits while a full-screen app is open"
    );
    assert_eq!(
        indexing_state(&held(ThrottleReason::Battery)),
        "Indexing waits until the computer is plugged in"
    );
    assert_eq!(
        indexing_state(&held(ThrottleReason::Busy)),
        "Indexing waits while the computer is busy"
    );
}
