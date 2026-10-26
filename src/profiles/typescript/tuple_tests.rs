use crate::config::smells::Smells;
use crate::noze::smells::detect_local;
use crate::report::{SmellFinding, SmellKind};
use crate::spine::parser::parse_file;
use std::fs;

fn tuple_packing_ts(source: &str, config: &Smells) -> Vec<SmellFinding> {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("sample.ts");
    fs::write(&path, source).unwrap();
    let file = parse_file(&path, 0).unwrap();
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
fn mixed_or_wide_tuples_still_pack_positional_data() {
    let findings = tuple_packing_ts(
        "export function load(): [string, number, boolean] {\n  return [\"a\", 1, true];\n}\n",
        &Smells::default(),
    );
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert!(findings[0].message.contains("labeled tuple"));

    let wide = "export function box(): [number, number, number, number, number] {\n  return [1, 2, 3, 4, 5];\n}\n";
    assert_eq!(tuple_packing_ts(wide, &Smells::default()).len(), 1);

    let strict = Smells {
        tuple_allow_homogeneous: false,
        ..Smells::default()
    };
    let rgb = "export function rgb(): [number, number, number] {\n  return [1, 2, 3];\n}\n";
    assert_eq!(tuple_packing_ts(rgb, &strict).len(), 1);
}
