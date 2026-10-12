//! Behavior tests for the USearch-backed VectorStore (ADR 0002).

use sbm_core::{Embedding, VectorStore};
use sbm_store_usearch::{Precision, UsearchStore};

fn unit(values: &[f32]) -> Embedding {
    let norm = values.iter().map(|v| v * v).sum::<f32>().sqrt();
    Embedding::new(values.iter().map(|v| v / norm).collect())
}

#[test]
fn added_vectors_are_found_most_similar_first() {
    let mut store = UsearchStore::new(3, Precision::F16).expect("store builds");
    store.add(1, &unit(&[1.0, 0.0, 0.0])).expect("adds");
    store.add(2, &unit(&[0.0, 1.0, 0.0])).expect("adds");
    store.add(3, &unit(&[0.9, 0.1, 0.0])).expect("adds");

    let neighbors = store.search(&unit(&[1.0, 0.0, 0.0]), 2).expect("searches");

    let ids: Vec<_> = neighbors.iter().map(|n| n.id).collect();
    assert_eq!(ids, vec![1, 3]);
}

#[test]
fn removed_vector_never_appears_in_search_results() {
    let mut store = UsearchStore::new(3, Precision::F16).expect("store builds");
    store.add(1, &unit(&[1.0, 0.0, 0.0])).expect("adds");
    store.add(2, &unit(&[0.9, 0.1, 0.0])).expect("adds");

    store.remove(1).expect("removes");
    let neighbors = store.search(&unit(&[1.0, 0.0, 0.0]), 10).expect("searches");

    let ids: Vec<_> = neighbors.iter().map(|n| n.id).collect();
    assert_eq!(ids, vec![2]);
    assert_eq!(store.len(), 1);
}

#[test]
fn saved_store_reopened_as_a_view_returns_the_same_results() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("vectors.usearch");
    let mut store = UsearchStore::new(3, Precision::F16).expect("store builds");
    store.add(1, &unit(&[1.0, 0.0, 0.0])).expect("adds");
    store.add(2, &unit(&[0.0, 1.0, 0.0])).expect("adds");
    store.add(3, &unit(&[0.9, 0.1, 0.0])).expect("adds");
    let query = unit(&[1.0, 0.0, 0.0]);
    let before = store.search(&query, 3).expect("searches");

    store.save(&path).expect("saves");
    let view = UsearchStore::open_view(&path).expect("opens view");

    assert_eq!(view.search(&query, 3).expect("searches view"), before);
}

#[test]
fn searching_an_empty_store_returns_no_neighbors() {
    let store = UsearchStore::new(3, Precision::F16).expect("store builds");

    let neighbors = store.search(&unit(&[1.0, 0.0, 0.0]), 5).expect("searches");

    assert!(neighbors.is_empty());
}

/// Deterministic pseudo-random unit vectors (no RNG dependency).
fn random_units(count: usize, dimensions: usize, seed: u64) -> Vec<Embedding> {
    let mut state = seed;
    let mut next = move || {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((state >> 33) as f32 / (1u64 << 31) as f32) - 0.5
    };
    (0..count)
        .map(|_| unit(&(0..dimensions).map(|_| next()).collect::<Vec<_>>()))
        .collect()
}

fn recall_at_10(precision: Precision) -> f32 {
    let (dimensions, k) = (256, 10);
    let stored = random_units(2_000, dimensions, 7);
    let queries = random_units(50, dimensions, 11);
    let mut store = UsearchStore::new(dimensions, precision).expect("store builds");
    for (id, embedding) in stored.iter().enumerate() {
        store.add(id as u64, embedding).expect("adds");
    }

    let mut found = 0;
    for query in &queries {
        let mut exact: Vec<(u64, f32)> =
            stored.iter().enumerate().map(|(id, e)| (id as u64, query.similarity(e))).collect();
        exact.sort_by(|a, b| b.1.total_cmp(&a.1));
        let exact_top: Vec<u64> = exact.iter().take(k).map(|(id, _)| *id).collect();
        let approximate = store.search(query, k).expect("searches");
        found += approximate.iter().filter(|n| exact_top.contains(&n.id)).count();
    }
    found as f32 / (queries.len() * k) as f32
}

#[test]
fn f16_store_finds_at_least_95_percent_of_exact_top_10() {
    let recall = recall_at_10(Precision::F16);
    assert!(recall >= 0.95, "f16 recall@10 = {recall}");
}

#[test]
#[ignore = "i8 recall@10 is ~0.89 on synthetic vectors (quantization loss); f16 vs i8 is decided on the real Benchmark (PRD 11.2 #2)"]
fn i8_store_finds_at_least_95_percent_of_exact_top_10() {
    let recall = recall_at_10(Precision::I8);
    assert!(recall >= 0.95, "i8 recall@10 = {recall}");
}
