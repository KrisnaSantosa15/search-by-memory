//! Measurement logic for embedding benchmarks and retrieval Benchmarks.

mod benchmark;
mod metrics;
mod retrieval;

pub use benchmark::{Benchmark, Query, Target};
pub use metrics::{FileRanking, FileResult, Scores};
pub use retrieval::{Chunk, ExactStore, RetrievalIndex, chunk_pages, chunk_text, evaluate};

use std::time::Duration;

/// Timings of repeated embedding calls, each embedding `batch_size` Chunks.
pub struct Throughput {
    batch_size: usize,
    timings: Vec<Duration>,
}

impl Throughput {
    pub fn new(batch_size: usize, timings: &[Duration]) -> Self {
        Self {
            batch_size,
            timings: timings.to_vec(),
        }
    }

    pub fn chunks_per_second(&self) -> f64 {
        let chunks = (self.batch_size * self.timings.len()) as f64;
        let seconds: f64 = self.timings.iter().map(Duration::as_secs_f64).sum();
        chunks / seconds
    }

    /// Per-batch latency at `percentile` (0.0–1.0), using the nearest-rank method.
    pub fn latency_percentile(&self, percentile: f64) -> Duration {
        let mut sorted = self.timings.clone();
        sorted.sort();
        let rank = (percentile * sorted.len() as f64).ceil().max(1.0) as usize;
        sorted[rank.min(sorted.len()) - 1]
    }
}
