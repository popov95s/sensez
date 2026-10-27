use crate::config::smells::Smells;
use crate::noze::smells::detect_local;
use crate::noze::smells::test_support as fixture;
use crate::report::{SmellFinding, SmellKind};

fn tuple_packing_ts(source: &str, config: &Smells) -> Vec<SmellFinding> {
    let file = fixture::parsed("sample.ts", source);
    detect_local(&file, config)
        .into_iter()
        .filter(|finding| finding.kind == SmellKind::TuplePacking)
        .collect()
}

#[test]
fn labeled_and_homogeneous_primitive_tuples_are_named_enough() {
    for source in [
        "export function parseColor(s: string): [number, number, number] {\n  return [1, 2, 3];\n}\n",
        "export function parseColor(s: string): [r: number, g: number, b: number] {\n  return [1, 2, 3];\n}\n",
    ] {
        let findings = tuple_packing_ts(source, &Smells::default());
        assert!(findings.is_empty(), "{source}\n{findings:?}");
    }
}

#[test]
fn mixed_tuple_packs_positional_data() {
    let findings = tuple_packing_ts(
        "export function load(): [string, number, boolean] {\n  return [\"a\", 1, true];\n}\n",
        &Smells::default(),
    );
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert!(
        findings[0].message.contains("labeled tuple"),
        "{}",
        findings[0].message
    );
}

#[test]
fn wide_homogeneous_tuple_packs_positional_data() {
    let wide = "export function box(): [number, number, number, number, number] {\n  return [1, 2, 3, 4, 5];\n}\n";
    let findings = tuple_packing_ts(wide, &Smells::default());
    assert_eq!(findings.len(), 1, "{findings:?}");
}

#[test]
fn homogeneous_tuple_is_reported_when_not_allowed() {
    let strict = Smells {
        tuple_allow_homogeneous: false,
        ..Smells::default()
    };
    let rgb = "export function rgb(): [number, number, number] {\n  return [1, 2, 3];\n}\n";
    let findings = tuple_packing_ts(rgb, &strict);
    assert_eq!(findings.len(), 1, "{findings:?}");
}
