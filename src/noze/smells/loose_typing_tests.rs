//! `loose_typing` precision and recall for both languages.

use super::test_support::local_of;
use crate::config::smells::{Smells, Strictness};
use crate::report::{SmellFinding, SmellKind};

fn loose(name: &str, body: &str, strictness: Strictness) -> Vec<SmellFinding> {
    let cfg = Smells {
        loose_typing_strictness: strictness,
        ..Smells::default()
    };
    local_of(name, body, &cfg, SmellKind::LooseTyping)
}

fn medium(name: &str, body: &str) -> Vec<SmellFinding> {
    loose(name, body, Strictness::Medium)
}

#[test]
fn type_predicate_returns_are_not_reported() {
    let body = "export const isRecord = (v: unknown): v is Record<string, unknown> => typeof v === \"object\";\nexport const isList = (v: unknown): v is unknown[] => Array.isArray(v);\n";
    assert!(medium("g.ts", body).is_empty());
}

#[test]
fn nullable_and_promised_typed_maps_match_the_bare_map() {
    let body = "export const a = (c: Record<string, number> | undefined) => c;\nexport const b = (c: Map<string, string> | null) => c;\nexport const d = async (ids: string[]): Promise<Map<string, string>> => new Map();\n";
    assert!(medium("m.ts", body).is_empty());
    assert_eq!(loose("m.ts", body, Strictness::High).len(), 3);
}

#[test]
fn unknown_returns_are_reported_only_at_high() {
    let body = "export const parse = (text: string): unknown => JSON.parse(text);\nexport const read = async (r: R): Promise<unknown> => parse(await r.text());\n";
    assert!(medium("u.ts", body).is_empty());
    assert_eq!(loose("u.ts", body, Strictness::High).len(), 2);
}

#[test]
fn thunk_returning_unknown_is_not_reported() {
    let body = "export async function busy(action: () => Promise<unknown>, done: () => unknown) { await action(); done(); }\n";
    assert!(medium("t.ts", body).is_empty());
}

#[test]
fn python_typed_dict_values_are_primitive_collections() {
    let body = "def a(c: dict[str, int]) -> int:\n    return 1\n\n\ndef b(c: Mapping[str, int]) -> int:\n    return 1\n\n\ndef c(c: dict[str, list[int]]) -> int:\n    return 1\n";
    assert!(medium("d.py", body).is_empty());
    assert_eq!(loose("d.py", body, Strictness::High).len(), 3);
}

#[test]
fn python_untyped_dict_values_stay_schema_erasing() {
    let body = "def a(c: dict[str, object]) -> int:\n    return 1\n\n\ndef b(c: dict) -> int:\n    return 1\n";
    assert_eq!(medium("d.py", body).len(), 2);
}

#[test]
fn destructured_any_param_is_reported() {
    let found = medium("p.ts", "export const pick = ({ id }: any) => id;\n");
    assert_eq!(found.len(), 1, "{found:?}");
}

#[test]
fn any_cast_in_a_body_is_left_to_type_escape_cast() {
    let body = "export function rows(input: string) {\n  const parsed = JSON.parse(input) as any;\n  return parsed.items;\n}\n";
    assert!(medium("c.ts", body).is_empty());
}

#[test]
fn any_class_field_is_reported() {
    let body = "export class Cache {\n  private store: any = {};\n  get(key: string) { return this.store[key]; }\n}\n";
    let found = medium("k.ts", body);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].line, 2);
}

#[test]
fn python_any_dataclass_field_is_reported() {
    let body = "@dataclass\nclass Job:\n    meta: dict[str, Any] = field(default_factory=dict)\n";
    assert_eq!(medium("j.py", body).len(), 1);
}

#[test]
fn python_any_kwargs_are_reported() {
    let body = "def run(**options: Any) -> None:\n    print(options)\n";
    assert_eq!(medium("r.py", body).len(), 1);
}
