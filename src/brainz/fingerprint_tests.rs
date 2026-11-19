use super::*;
use serde_json::json;

#[test]
fn cycle_identity_ignores_module_order() {
    let a = cycle_key(&json!({"modules": ["x", "y"]}));
    let b = cycle_key(&json!({"modules": ["y", "x"]}));
    assert_eq!(a.hash, b.hash);
    assert_eq!(a.detector, Detector::Cycles);
}
