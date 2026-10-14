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

#[test]
fn bytes_already_on_disk_do_not_count_as_speed() {
    let time_left = TimeLeft::starting_at(50 * MB);

    let estimate = time_left.estimate(
        Progress {
            done: 60 * MB,
            total: 100 * MB,
        },
        Duration::from_secs(10),
    );

    assert_eq!(estimate, Some(Duration::from_secs(40)));
}

#[test]
fn no_estimate_before_the_speed_is_known() {
    let time_left = TimeLeft::starting_at(0);
    let progress = Progress {
        done: MB,
        total: 100 * MB,
    };

    assert_eq!(time_left.estimate(progress, Duration::from_secs(1)), None);
    let stalled = Progress {
        done: 0,
        ..progress
    };
    assert_eq!(time_left.estimate(stalled, Duration::from_secs(30)), None);
}
