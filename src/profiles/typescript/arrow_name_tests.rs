//! Names given to arrow functions and function expressions.

use super::tests::findings_for;
use super::TsProfile;
use crate::config::smells::Smells;
use crate::report::SmellKind;
use crate::spine::parser::parse_source;

#[test]
fn nested_function_in_arrow_names_parent_binding() {
    let body = "  let x = 0;\n".repeat(17);
    let src = format!(
        "const useThing = () => {{\n  function inner() {{\n{body}  }}\n  return inner;\n}};\n"
    );
    let finding = findings_for(src.as_bytes(), &Smells::default())
        .into_iter()
        .find(|f| f.kind == SmellKind::HeavyNestedFunction)
        .expect("must flag heavy nested function");
    assert!(
        finding.message.contains("inside `useThing`"),
        "{}",
        finding.message
    );
}

#[test]
fn arrow_params_feed_type_assisted_smells() {
    let src = b"export function declFlags(a: boolean, b: boolean, c: boolean) { return a && b && c; }\nexport const arrowFlags = (a: boolean, b: boolean, c: boolean) => a && b && c;\nexport const g = (x: any) => x;\n";
    let findings = findings_for(src, &Smells::default());
    let bools: Vec<_> = findings
        .iter()
        .filter(|f| f.kind == SmellKind::BooleanBlindness)
        .map(|f| f.symbol.as_str())
        .collect();
    assert_eq!(bools, vec!["declFlags", "arrowFlags"], "{findings:?}");
    assert!(
        findings
            .iter()
            .any(|f| f.kind == SmellKind::LooseTyping && f.symbol == "g"),
        "{findings:?}"
    );
}

#[test]
fn same_named_methods_keep_their_own_param_types() {
    let src = b"class A {\n  run(a: boolean, b: boolean, c: boolean) { return a; }\n}\nclass B {\n  run(a: string, b: string, c: string) { return a; }\n}\n";
    let lines: Vec<_> = findings_for(src, &Smells::default())
        .into_iter()
        .filter(|f| f.kind == SmellKind::BooleanBlindness)
        .map(|f| (f.symbol, f.line))
        .collect();
    assert_eq!(lines, vec![("run".to_string(), 2)]);
}

#[test]
fn callback_arrows_are_named_after_their_call_site() {
    let src = b"describe(\"rows\", () => {\n  it(\"adds rows\", () => {});\n  it(\"drops rows\", () => {});\n  items.forEach((x) => x);\n});\n";
    let names: Vec<_> = parse_source(src, 0, "m", &TsProfile)
        .unwrap()
        .units
        .functions
        .into_iter()
        .map(|f| f.name)
        .collect();
    assert_eq!(
        names,
        vec![
            "describe(\"rows\") callback",
            "it(\"adds rows\") callback",
            "it(\"drops rows\") callback",
            "items.forEach callback",
        ]
    );
}

fn function_names(src: &[u8]) -> Vec<String> {
    parse_source(src, 0, "m", &TsProfile)
        .unwrap()
        .units
        .functions
        .into_iter()
        .map(|f| f.name)
        .collect()
}

#[test]
fn quoted_pair_keys_name_their_arrow_without_quotes() {
    let names =
        function_names(b"export const handlers = { \"on-save\": () => 1, plain: () => 2 };\n");
    assert_eq!(names, vec!["on-save", "plain"]);
}

#[test]
fn computed_pair_keys_leave_their_arrow_unnamed() {
    let names = function_names(b"export const it = { [Symbol.iterator]: () => 1 };\n");
    assert_eq!(names, vec![""]);
}
