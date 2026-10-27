use super::detect_local;
use super::test_support as fixture;
use crate::config::smells::{Smells, Strictness};
use crate::report::{SmellFinding, SmellKind};

fn magic(ext: &str, source: &str, cfg: &Smells) -> Vec<SmellFinding> {
    let file = fixture::parsed(&format!("sample.{ext}"), source);
    detect_local(&file, cfg)
        .into_iter()
        .filter(|finding| finding.kind == SmellKind::MagicStringDefault)
        .collect()
}

fn fires(ext: &str, source: &str) -> bool {
    !magic(ext, source, &Smells::default()).is_empty()
}

#[test]
fn consumed_and_rendered_fallbacks_are_not_sentinels() {
    for source in [
        "export const cls = (on: boolean) => [\"row\", on ? \"row--on\" : \"\"].join(\" \");\n",
        "export const plural = (n: number) => `${n} item${n === 1 ? \"\" : \"s\"}`;\n",
        "export const size = (big: boolean) => (big ? \"l\" : \"s\");\n",
        "export const byName = (a: { name?: string }, b: { name?: string }) => (a.name ?? \"\").localeCompare(b.name ?? \"\");\n",
        "export const greet = (u: { name?: string }) => `Hello ${u.name ?? \"\"}`;\n",
    ] {
        assert!(!fires("ts", source), "{source}");
    }
    for source in [
        "export const Cell = (p: { v?: string }) => <td>{p.v ?? \"—\"}</td>;\n",
        "export function Field(p: { v?: string }) { const [v] = useState(p.v ?? \"\"); return <input value={v} />; }\n",
        "export const Name = (p: { n?: string }) => <b>{p.n ?? \"Unknown\"}</b>;\n",
    ] {
        assert!(!fires("tsx", source), "{source}");
    }
}

#[test]
fn stored_compared_and_returned_sentinels_fire() {
    for source in [
        "export async function link(parent: { id: string } | undefined) { await save({ parentId: parent?.id ?? \"\" }); }\n",
        "export const isAdmin = (u: { role?: string }) => (u.role ?? \"\") === \"admin\";\n",
        "export function coerce(name?: string): string { return name || \"\"; }\n",
        "export function pick(name?: string): string { return name ? name : \"?\"; }\n",
    ] {
        assert!(fires("ts", source), "{source}");
    }
}

#[test]
fn sentinel_words_and_bracketed_placeholders_fire_when_stored() {
    for source in [
        "export async function tag(o: { label?: string }) { await save({ label: o.label ?? \"n/a\" }); }\n",
        "export async function title(o: { t?: string }) { await save({ title: o.t ?? \"(unnamed)\" }); }\n",
    ] {
        assert!(fires("ts", source), "{source}");
    }
}

#[test]
fn message_names_the_sink_and_first_hit_in_typescript_syntax() {
    let source = "export async function link(p?: { id: string }) {\n  await save({ parentId: p?.id ?? \"\" });\n}\n";
    let finding = magic("ts", source, &Smells::default())
        .pop()
        .expect("stored sentinel must fire");
    assert_eq!(finding.line, 2);
    assert!(
        finding.message.contains("stored in `parentId`"),
        "{}",
        finding.message
    );
    assert!(finding.message.contains("line 2"), "{}", finding.message);
    assert!(finding.message.contains("`?? \"\"`"), "{}", finding.message);
    assert!(!finding.message.contains("or \"\"`"), "{}", finding.message);
}

#[test]
fn strictness_moves_the_sink_filter() {
    let returned = "export function coerce(name?: string): string { return name || \"\"; }\n";
    let rendered = "export const greet = (u: { name?: string }) => `Hello ${u.name ?? \"\"}`;\n";
    let low = Smells {
        magic_string_default_strictness: Strictness::Low,
        ..Smells::default()
    };
    let high = Smells {
        magic_string_default_strictness: Strictness::High,
        ..Smells::default()
    };
    assert!(magic("ts", returned, &low).is_empty());
    assert!(!magic("ts", rendered, &high).is_empty());
}

#[test]
fn python_keeps_short_fallbacks_but_requires_a_self_guard() {
    assert!(fires("py", "def f(name=None):\n    return name or \"\"\n"));
    assert!(fires(
        "py",
        "def f(name=None):\n    return name if name else \"\"\n"
    ));
    assert!(!fires(
        "py",
        "def f(n):\n    return \"s\" if n > 1 else \"\"\n"
    ));
}

#[test]
fn typescript_undefined_is_a_sentinel_word() {
    assert!(fires(
        "ts",
        "export async function tag(o: { l?: string }) { await save({ l: o.l ?? \"undefined\" }); }\n",
    ));
}

#[test]
fn python_sentinel_words_fire_when_stored() {
    assert!(fires(
        "py",
        "def label(store, order=None):\n    store.save({\"label\": order or \"n/a\"})\n"
    ));
}
