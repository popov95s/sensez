//! `literal_membership`: outside vocabularies stay quiet, or-chains are categories.

use super::test_support::local_of;
use crate::config::smells::Smells;
use crate::report::{SmellFinding, SmellKind};

fn membership(name: &str, body: &str) -> Vec<SmellFinding> {
    local_of(name, body, &Smells::default(), SmellKind::LiteralMembership)
}

#[test]
fn punctuation_sets_are_external() {
    let ts = "export const sep = (c: string) => [\" \", \".\", \":\", \"[\"].includes(c);\n";
    let py = "def sep(ch):\n    return ch in (\" \", \",\", \";\")\n";
    assert!(membership("s.ts", ts).is_empty());
    assert!(membership("s.py", py).is_empty());
}

#[test]
fn markup_tag_names_are_external() {
    let ts = "export const block = (n: N) => n.kids.some((k) => [\"p\", \"tbl\", \"tr\"].includes(local(k)));\nexport const isCell = (el: E) => [\"td\", \"th\"].includes(el.tagName);\n";
    let py = "def is_block(tag):\n    return tag in (\"p\", \"table\", \"tr\")\n";
    assert!(membership("m.ts", ts).is_empty());
    assert!(membership("m.py", py).is_empty());
}

#[test]
fn type_guard_callback_list_is_quiet() {
    let ts = "export const grades = (xs: string[]) => xs.filter((v): v is Grade => [\"A\", \"B\", \"C\"].includes(v));\n";
    assert!(membership("g.ts", ts).is_empty());
}

#[test]
fn or_chain_of_string_equalities_is_a_category() {
    let ts = "export function done(state: string) {\n  return state === \"done\" || state === \"failed\" || state === \"cancelled\";\n}\n";
    let found = membership("o.ts", ts);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].line, 2);
}

#[test]
fn python_or_chain_of_string_equalities_is_a_category() {
    let py = "def done(state):\n    return state == \"done\" or state == \"failed\" or state == \"cancelled\"\n";
    assert_eq!(membership("o.py", py).len(), 1);
}

#[test]
fn two_equalities_or_mixed_operands_are_not_a_category() {
    let ts = "export const a = (s: string) => s === \"x\" || s === \"y\";\nexport const b = (s: string, t: string) => s === \"x\" || t === \"y\" || s === \"z\";\n";
    assert!(membership("o.ts", ts).is_empty());
}
