//! File-view retrieval metrics over Benchmark Queries.

use std::path::PathBuf;

use crate::Query;

/// Where a Query's Target files appear in a File-view result list.
pub struct FileRanking {
    /// 1-based rank of the first Result that is a Target file.
    first_target_rank: Option<usize>,
}

impl FileRanking {
    /// Ranks `ranked_files`, the File-view Results most relevant first, against `query`.
    pub fn new(query: &Query, ranked_files: &[PathBuf]) -> Self {
        let first_target_rank = ranked_files
            .iter()
            .position(|file| query.targets.iter().any(|target| &target.file == file))
            .map(|index| index + 1);
        Self { first_target_rank }
    }

    fn found_within(&self, k: usize) -> bool {
        self.first_target_rank.is_some_and(|rank| rank <= k)
    }
}

/// Retrieval quality across all Benchmark Queries.
pub struct Scores {
    rankings: Vec<FileRanking>,
}

impl Scores {
    pub fn new(rankings: Vec<FileRanking>) -> Self {
        Self { rankings }
    }

    /// Share of Queries whose Target file is among the top `k` Results.
    pub fn recall_at(&self, k: usize) -> f64 {
        let found = self.rankings.iter().filter(|ranking| ranking.found_within(k)).count();
        found as f64 / self.rankings.len() as f64
    }

    /// Mean of 1 / rank of the first Target file; 0 when it is not found.
    pub fn mean_reciprocal_rank(&self) -> f64 {
        let total: f64 = self
            .rankings
            .iter()
            .filter_map(|ranking| ranking.first_target_rank)
            .map(|rank| 1.0 / rank as f64)
            .sum();
        total / self.rankings.len() as f64
    }
}
