//! `mutated_parameter`: private collectors, mutation shapes and anchors.

use super::test_support::local_of;
use crate::config::smells::Smells;
use crate::report::{SmellFinding, SmellKind};

fn mutated(name: &str, body: &str) -> Vec<SmellFinding> {
    local_of(name, body, &Smells::default(), SmellKind::MutatedParameter)
}

#[test]
fn private_collector_with_leading_collection_param_is_quiet() {
    let body = "const appendTo = (groups: Map<string, number[]>, key: string, i: number) => {\n  groups.set(key, [i]);\n};\nconst addTrimmed = (into: Set<string>, value: string) => {\n  into.add(value.trim());\n};\nexport function g(words: string[]) { const m = new Map<string, number[]>(); const s = new Set<string>(); words.forEach((w, i) => { appendTo(m, w, i); addTrimmed(s, w); }); return { m, s }; }\n";
    assert!(mutated("a.ts", body).is_empty());
}

#[test]
fn python_private_collector_with_leading_dict_param_is_quiet() {
    let body = "def _add_word(index: dict[str, list[int]], word: str, position: int) -> None:\n    index.setdefault(word[:1], []).append(position)\n    index[word] = [position]\n";
    assert!(mutated("a.py", body).is_empty());
}

#[test]
fn exported_collector_still_fires() {
    let body = "export const appendTo = (groups: Map<string, number[]>, key: string) => {\n  groups.set(key, []);\n};\n";
    assert_eq!(mutated("a.ts", body).len(), 1);
}

#[test]
fn object_assign_on_a_param_is_a_mutation() {
    let body =
        "export function discount(order: Order) {\n  Object.assign(order, { total: 1 });\n}\n";
    assert_eq!(mutated("o.ts", body).len(), 1);
}

#[test]
fn deleting_a_param_property_is_a_mutation() {
    let body = "export function dropTotal(order: Partial<Order>) {\n  delete order.total;\n}\n";
    assert_eq!(mutated("d.ts", body).len(), 1);
}

#[test]
fn mutating_a_destructured_param_is_a_mutation() {
    let body = "export const sortInPlace = ({ items }: Order) => {\n  items.sort();\n};\n";
    let found = mutated("s.ts", body);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].message.contains("items"), "{}", found[0].message);
}

#[test]
fn finding_is_anchored_at_the_first_mutation() {
    let body = "export function tag(rows: string[], extra: string[]) {\n  const n = rows.length;\n  if (n > 3) {\n    rows.push(\"x\");\n  }\n  extra.sort();\n}\n";
    let found = mutated("t.ts", body);
    assert_eq!(found[0].line, 4);
    assert_eq!(found[0].lines, vec![4, 6]);
}
