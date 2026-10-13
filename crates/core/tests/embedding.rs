use sbm_core::Embedding;

#[test]
fn truncated_embedding_keeps_leading_values_rescaled_to_unit_length() {
    let embedding = Embedding::new(vec![3.0, 4.0, 12.0]);

    assert_eq!(embedding.truncated(2), Embedding::new(vec![0.6, 0.8]));
}
