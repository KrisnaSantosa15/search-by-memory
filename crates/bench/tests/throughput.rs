use std::time::Duration;

use sbm_bench::Throughput;

#[test]
fn chunks_per_second_is_total_chunks_over_total_time() {
    let throughput = Throughput::new(8, &[Duration::from_millis(500), Duration::from_millis(1500)]);

    assert_eq!(throughput.chunks_per_second(), 8.0);
}
