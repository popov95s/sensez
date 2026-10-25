use super::test_support::local_of;
use crate::config::smells::{Smells, Strictness};
use crate::report::{SmellFinding, SmellKind};

fn magic(name: &str, source: &str, strictness: Strictness) -> Vec<SmellFinding> {
    let cfg = Smells {
        magic_string_default_strictness: strictness,
        ..Smells::default()
    };
    local_of(name, source, &cfg, SmellKind::MagicStringDefault)
}

fn medium(name: &str, source: &str) -> Vec<SmellFinding> {
    magic(name, source, Strictness::Medium)
}

#[test]
fn coercion_and_lookup_arguments_are_string_operations() {
    for source in [
        "export const page = (q: { p?: string }) => Number(q.p ?? \"0\");\n",
        "export const limit = (q: { l?: string }) => parseInt(q.l ?? \"20\", 10);\n",
        "export const since = (q: { s?: string }) => Date.parse(q.s ?? \"\");\n",
        "export const ok = (i: { s?: string }) => SLUG.test(i.s ?? \"\");\n",
        "export const taken = (i: { s?: string }) => RESERVED.has(i.s ?? \"\");\n",
        "export const label = (m: Map<string, string>, k?: string) => m.get(k ?? \"\");\n",
    ] {
        assert!(medium("q.ts", source).is_empty(), "{source}");
    }
}

#[test]
fn call_inside_a_template_is_rendered() {
    let source = "export const row = (r: { o?: string }) => `| ${esc(r.o ?? \"—\")} |`;\n";
    assert!(medium("row.ts", source).is_empty());
}

#[test]
fn translation_options_are_rendered() {
    let source =
        "export const hi = (u: { n?: string }) => t(\"greeting\", { name: u.n ?? \"\" });\n";
    assert!(medium("hi.ts", source).is_empty());
}

#[test]
fn ternary_branch_inherits_the_outer_sink() {
    let source = "export const text = (i: { b?: string[]; s?: string }) => (i.b ? i.b.join(\" \") : (i.s ?? \"\")).trim();\n";
    assert!(medium("text.ts", source).is_empty());
}

#[test]
fn message_names_the_callee_a_fallback_is_passed_to() {
    let source =
        "export async function link(p?: { id: string }) {\n  await send(p?.id ?? \"\");\n}\n";
    let found = medium("link.ts", source);
    assert!(
        found[0].message.contains("passed to `send`"),
        "{}",
        found[0].message
    );
}

#[test]
fn python_strictness_levels_differ() {
    let source = "def greet(u):\n    return f\"Hi {u.name or '-'}\"\n\n\ndef slug(u):\n    return (u.handle or \"\").strip()\n\n\ndef show(u):\n    print(u.nick or \"-\")\n";
    assert!(magic("r.py", source, Strictness::Low).is_empty());
    assert!(magic("r.py", source, Strictness::Medium).is_empty());
    assert_eq!(magic("r.py", source, Strictness::High).len(), 3);
}

#[test]
fn python_coercion_argument_is_a_string_operation() {
    let source = "def page(params):\n    return int(params.get(\"page\") or \"0\")\n";
    assert!(medium("p.py", source).is_empty());
}

#[test]
fn python_stored_fallback_fires_at_low() {
    let source =
        "def save_owner(store, p):\n    store.save({\"owner\": p.get(\"owner\") or \"unknown\"})\n";
    let found = magic("s.py", source, Strictness::Low);
    assert_eq!(found.len(), 1);
    assert!(
        found[0].message.contains("stored in `owner`"),
        "{}",
        found[0].message
    );
}

#[test]
fn python_get_default_that_is_compared_fires() {
    let source = "def region_of(payload):\n    region = payload.get(\"region\", \"\")\n    if region == \"\":\n        return \"none\"\n    return region\n";
    assert_eq!(medium("g.py", source).len(), 1);
}

#[test]
fn python_getattr_default_fires() {
    let source = "def owner(o, store):\n    store.put(getattr(o, \"owner\", \"\"))\n";
    assert_eq!(medium("g.py", source).len(), 1);
}

#[test]
fn destructuring_default_compared_to_itself_fires() {
    let source = "export function status(row: R) {\n  const { status = \"\" } = row;\n  if (status === \"\") return \"missing\";\n  return status;\n}\n";
    let found = medium("s.ts", source);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].line, 2);
}

#[test]
fn parameter_default_compared_to_itself_fires() {
    let source =
        "export function label(name: string = \"\") {\n  return name === \"\" ? null : name;\n}\n";
    assert_eq!(medium("l.ts", source).len(), 1);
}

#[test]
fn parameter_default_that_is_never_compared_is_quiet() {
    let source = "export function join(parts: string[], sep: string = \"\") {\n  return parts.join(sep);\n}\n";
    assert!(medium("j.ts", source).is_empty());
}
