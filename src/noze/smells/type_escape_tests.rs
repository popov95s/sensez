use crate::config::smells::{Smells, Strictness};
use crate::noze::smells::test_support::local_of;
use crate::report::{SmellFinding, SmellKind};

fn escapes(name: &str, body: &str, strictness: Strictness) -> Vec<SmellFinding> {
    let cfg = Smells {
        type_escape_strictness: strictness,
        ..Smells::default()
    };
    local_of(name, body, &cfg, SmellKind::TypeEscapeCast)
}

fn medium(name: &str, body: &str) -> Vec<SmellFinding> {
    escapes(name, body, Strictness::Medium)
}

#[test]
fn as_any_in_a_body_is_reported_with_lines_and_density() {
    let body = "export function total(res: Response) {\n  const a = res as any;\n  const b = a.items as any;\n  return (b as unknown as number[]).length;\n}\n";
    let found = medium("cart.ts", body);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].lines, vec![2, 3, 4]);
    assert!(
        found[0]
            .message
            .starts_with("3 type escapes in 5 lines: `as any` at L2, L3; `as unknown as` at L4"),
        "{}",
        found[0].message
    );
}

#[test]
fn angle_bracket_any_is_reported() {
    let found = medium(
        "legacy.ts",
        "export function read(v: unknown) {\n  return (<any>v).name;\n}\n",
    );
    assert_eq!(found.len(), 1);
}

#[test]
fn as_const_and_satisfies_are_never_escapes() {
    let body = "export function modes() {\n  const m = ['a', 'b'] as const;\n  const c = { port: 1 } satisfies Config;\n  return [m, c];\n}\n";
    assert!(medium("modes.ts", body).is_empty());
}

#[test]
fn plain_narrowing_cast_is_not_an_escape() {
    let body = "export function name(v: unknown) {\n  return (v as User).name;\n}\n";
    assert!(medium("user.ts", body).is_empty());
}

#[test]
fn ts_ignore_without_reason_is_reported_and_with_reason_is_quiet() {
    let bare = "export function f(x: number) {\n  // @ts-ignore\n  return x.foo;\n}\n";
    assert_eq!(medium("bare.ts", bare).len(), 1);
    let reasoned = "export function f(x: number) {\n  // @ts-expect-error: upstream typings miss the overload\n  return x.foo;\n}\n";
    assert!(medium("reasoned.ts", reasoned).is_empty());
}

#[test]
fn guarded_subject_in_a_type_predicate_is_not_an_escape() {
    let body = "export function isUser(value: unknown): value is User {\n  return typeof (value as any).name === 'string';\n}\n";
    assert!(medium("guard.ts", body).is_empty());
}

#[test]
fn test_sources_are_quiet() {
    let body = "export function build() {\n  return {} as any;\n}\n";
    assert!(medium("cart.test.ts", body).is_empty());
}

#[test]
fn low_strictness_skips_double_casts() {
    let body = "export function f(v: Shape) {\n  return v as unknown as Circle;\n}\n";
    assert!(escapes("shape.ts", body, Strictness::Low).is_empty());
    assert_eq!(medium("shape.ts", body).len(), 1);
}

#[test]
fn non_null_call_result_is_reported_only_at_high() {
    let body = "export function first(xs: Item[]) {\n  return xs.find((x) => x.ok)!.id;\n}\n";
    assert!(medium("items.ts", body).is_empty());
    assert_eq!(escapes("items.ts", body, Strictness::High).len(), 1);
}

#[test]
fn python_bare_type_ignore_is_reported_and_coded_one_is_quiet() {
    let bare = "def load(path):\n    data = read(path)  # type: ignore\n    return data\n";
    assert_eq!(medium("load.py", bare).len(), 1);
    let coded =
        "def load(path):\n    data = read(path)  # type: ignore[arg-type]\n    return data\n";
    assert!(medium("load.py", coded).is_empty());
}

#[test]
fn python_cast_to_any_is_reported() {
    let body = "from typing import Any, cast\n\ndef load(raw):\n    payload = cast(Any, raw)\n    return payload['id']\n";
    let found = escapes("load.py", body, Strictness::Low);
    assert_eq!(found.len(), 1);
    assert!(
        found[0].message.contains("`cast(Any, …)` at L4"),
        "{}",
        found[0].message
    );
}

#[test]
fn cast_with_a_rationale_comment_on_its_line_is_quiet() {
    let body = "export function send(opts: Options) {\n  return client.post(opts as any); // workaround because the upstream typings miss the overload, see https://example.com/issue/1\n}\n";
    assert!(medium("send.ts", body).is_empty());
}

#[test]
fn non_null_call_checked_by_its_condition_is_quiet() {
    let body = "export function label(e: Entry) {\n  return lookup(e, 'name') ? { name: lookup(e, 'name')! } : {};\n}\n";
    assert!(escapes("entry.ts", body, Strictness::High).is_empty());
}

#[test]
fn map_lookup_after_a_has_check_is_quiet() {
    let body = "export function bucket(m: Map<string, string[]>, k: string) {\n  if (!m.has(k)) m.set(k, []);\n  return m.get(k)!;\n}\n";
    assert!(escapes("bucket.ts", body, Strictness::High).is_empty());
}

#[test]
fn drain_inside_a_length_loop_is_quiet() {
    let body =
        "export function drain(xs: number[]) {\n  while (xs.length > 0) use(xs.pop()!);\n}\n";
    assert!(escapes("drain.ts", body, Strictness::High).is_empty());
}

#[test]
fn cast_under_a_rationale_comment_is_quiet() {
    let body = "export function fake(): Ctx {\n  const ctx = { run: () => 1 };\n  // The caller only reaches run, so the rest of the context stays unimplemented on purpose.\n  return ctx as unknown as Ctx;\n}\n";
    assert!(medium("fake.ts", body).is_empty());
}
