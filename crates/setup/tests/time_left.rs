//! How long a download still takes, from how fast it has gone so far.

use std::time::Duration;

use sbm_setup::{Progress, TimeLeft};

const MB: u64 = 1_000_000;

#[test]
fn time_left_follows_the_speed_so_far() {
    let time_left = TimeLeft::starting_at(0);

    let estimate = time_left.estimate(
        Progress {
            done: 10 * MB,
            total: 70 * MB,
        },
        Duration::from_secs(10),
    );

    assert_eq!(estimate, Some(Duration::from_secs(60)));
}
