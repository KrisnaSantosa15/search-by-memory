use std::time::Duration;

use sbm_bench::Throughput;

#[test]
fn chunks_per_second_is_total_chunks_over_total_time() {
    let throughput = Throughput::new(
        8,
        &[Duration::from_millis(500), Duration::from_millis(1500)],
    );

    assert_eq!(throughput.chunks_per_second(), 8.0);
}

#[test]
fn latency_percentile_uses_nearest_rank() {
    let timings: Vec<Duration> = (1..=20).map(Duration::from_millis).collect();
    let throughput = Throughput::new(1, &timings);

    assert_eq!(
        throughput.latency_percentile(0.50),
        Duration::from_millis(10)
    );
    assert_eq!(
        throughput.latency_percentile(0.95),
        Duration::from_millis(19)
    );
}
