use super::*;

#[test]
fn stale_model_or_schema_drops_vectors() {
    let mut cache = SemanticCache {
        schema_version: 0,
        model_id: "old-model".to_string(),
        vectors: vec![CachedVector {
            key: 1,
            vector: vec![1.0],
        }],
    };

    cache.reset_if_stale();

    assert_eq!(cache.schema_version, SCHEMA_VERSION);
    assert_eq!(cache.model_id, MODEL_ID);
    assert!(cache.vectors.is_empty());
}
