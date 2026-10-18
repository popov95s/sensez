//! TypeScript `loose_typing` precision: guards, `unknown`, typed maps, overloads.

use crate::config::smells::{Smells, Strictness};
use crate::noze::smells::detect_local;
use crate::report::{Severity, SmellFinding, SmellKind};
use crate::spine::parser::parse_file;
use std::fs;

fn loose(body: &str, cfg: &Smells) -> Vec<SmellFinding> {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("types.ts");
    fs::write(&path, body).unwrap();
    detect_local(&parse_file(&path, 0).unwrap(), cfg)
        .into_iter()
        .filter(|f| f.kind == SmellKind::LooseTyping)
        .collect()
}

fn flagged(body: &str) -> Vec<String> {
    loose(body, &Smells::default())
        .into_iter()
        .map(|f| f.symbol)
        .collect()
}

#[test]
fn guards_errors_typed_maps_and_ignored_params_are_not_loose() {
    let body = "\
interface User { id: string }
export function isUser(value: unknown): value is User { return value !== null; }
export function assertUser(value: unknown): asserts value is User {}
export function toMessage(error: unknown): string { return String(error); }
export function countWords(counts: Map<string, number>): number { return counts.size; }
export function labels(map: Record<string, string>): string[] { return Object.keys(map); }
export function noop(_ctx: unknown): void {}
export function parse(input: string): User;
export function parse(input: number): User;
export function parse(input: unknown): User { return input as User; }
";
    assert!(flagged(body).is_empty(), "{:?}", flagged(body));
}

#[test]
fn escape_hatches_and_schema_erasing_types_still_flag() {
    let body = "\
export function anyBag(x: any): void {}
export function readProp(obj: object): unknown { return obj; }
export function withOverrides(o: Record<string, unknown>): void {}
export function f(m: Map<string, any>): void {}
";
    let findings = loose(body, &Smells::default());
    let symbols: Vec<&str> = findings.iter().map(|f| f.symbol.as_str()).collect();
    assert_eq!(symbols, vec!["anyBag", "readProp", "withOverrides", "f"]);
    let read_prop = &findings[1];
    assert_eq!(
        read_prop.metric, 2,
        "param and return: {}",
        read_prop.message
    );
}

#[test]
fn high_strictness_reports_bare_unknown_with_narrowing_advice() {
    let cfg = Smells {
        loose_typing_strictness: Strictness::High,
        ..Smells::default()
    };
    let findings = loose(
        "export function toMessage(error: unknown): string { return String(error); }\n",
        &cfg,
    );
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, Severity::Info);
    assert!(
        findings[0].message.contains("type guard or schema parse"),
        "{}",
        findings[0].message
    );
}

#[test]
fn no_explicit_any_directive_exempts_the_signature() {
    let body = "\
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function onSpan(span: any): void {}
export function g(
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  span: any,
  other: any,
): void {}
";
    let findings = loose(body, &Smells::default());
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert!(findings[0].message.contains("other: any"));
    assert!(!findings[0].message.contains("span"));
}
