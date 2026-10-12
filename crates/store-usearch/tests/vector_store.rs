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
