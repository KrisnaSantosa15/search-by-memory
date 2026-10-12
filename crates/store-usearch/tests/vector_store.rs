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
