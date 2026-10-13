//! File-view retrieval metrics over Benchmark Queries.

use std::collections::HashSet;
use std::path::PathBuf;

use sbm_core::Location;

use crate::Query;

/// One File-view Result: a file and the Location of the Snippet shown for it, if any.
#[derive(Debug, Clone, PartialEq)]
pub struct FileResult {
    pub file: PathBuf,
    /// `None` when the file was found by name only.
    pub snippet: Option<Location>,
}

/// Where a Query's Target files appear in a File-view result list.
pub struct FileRanking {
    /// 1-based ranks of the Results that are Target files, best first.
    target_ranks: Vec<usize>,
    /// Number of distinct Target files.
    target_files: usize,
}

impl FileRanking {
    /// Ranks `results`, the File-view Results most relevant first, against `query`.
    pub fn new(query: &Query, results: &[FileResult]) -> Self {
        let targets: HashSet<&PathBuf> = query.targets.iter().map(|target| &target.file).collect();
        let target_ranks = results
            .iter()
            .enumerate()
            .filter(|(_, result)| targets.contains(&result.file))
            .map(|(index, _)| index + 1)
            .collect();
        Self {
            target_ranks,
            target_files: targets.len(),
        }
    }

    fn first_target_rank(&self) -> Option<usize> {
        self.target_ranks.first().copied()
    }

    fn found_within(&self, k: usize) -> bool {
        self.first_target_rank().is_some_and(|rank| rank <= k)
    }

    /// Discounted gain of Target files in the top `k`, relative to ranking them all first.
    fn ndcg_at(&self, k: usize) -> f64 {
        let gain = |rank: usize| 1.0 / ((rank + 1) as f64).log2();
        let found: f64 = self
            .target_ranks
            .iter()
            .filter(|&&rank| rank <= k)
            .map(|&rank| gain(rank))
            .sum();
        let ideal: f64 = (1..=self.target_files.min(k)).map(gain).sum();
        found / ideal
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
        let found = self
            .rankings
            .iter()
            .filter(|ranking| ranking.found_within(k))
            .count();
        found as f64 / self.rankings.len() as f64
    }

    /// Mean of 1 / rank of the first Target file; 0 when it is not found.
    pub fn mean_reciprocal_rank(&self) -> f64 {
        self.mean(|ranking| {
            ranking
                .first_target_rank()
                .map_or(0.0, |rank| 1.0 / rank as f64)
        })
    }

    /// Mean normalized discounted cumulative gain over the top `k` Results.
    pub fn ndcg_at(&self, k: usize) -> f64 {
        self.mean(|ranking| ranking.ndcg_at(k))
    }

    fn mean(&self, score: impl Fn(&FileRanking) -> f64) -> f64 {
        self.rankings.iter().map(score).sum::<f64>() / self.rankings.len() as f64
    }
}
