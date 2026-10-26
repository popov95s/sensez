//! Repeated guards and defensive fallbacks.

use super::review_risks_tests::{has, local};
use crate::report::SmellKind;

#[test]
fn guards_reset_across_await_and_sibling_loops() {
    let awaited = r#"
export async function rv(c) {
  if (c.v) return;
  await work();
  if (c.v) return;
}
"#;
    assert!(!has(&local("ts", awaited), SmellKind::RedundantValidation));

    let loops = r#"
export function rv2(xs) {
  for (const x of xs) if (x > 3) continue;
  for (const x of xs) {
    if (x > 3) continue;
    use(x);
  }
}
"#;
    assert!(!has(&local("ts", loops), SmellKind::RedundantValidation));

    let python = "async def rv(c):\n    if c.v:\n        return\n    await work()\n    if c.v:\n        return\n";
    assert!(!has(&local("py", python), SmellKind::RedundantValidation));
}

#[test]
fn guards_on_reassigned_names_are_forgotten() {
    let source = r#"
export function next(v) {
  if (!v) return;
  v = advance(v);
  if (!v) return;
  return v;
}
"#;
    assert!(!has(&local("ts", source), SmellKind::RedundantValidation));
}

#[test]
fn repeated_guard_on_one_path_still_fires() {
    let source = r#"
export function name(u, other) {
  let k;
  k = 1;
  k = 2;
  if (!u) throw E;
  const n = u.name;
  if (!u) throw E;
  return n + k + other;
}
"#;
    assert!(has(&local("ts", source), SmellKind::RedundantValidation));
}

#[test]
fn fallbacks_outside_the_try_are_not_defensive() {
    let normalised = r#"
export function norm(raw) {
  const names = raw.names ?? [];
  const opts = raw.opts ?? {};
  try {
    return parse(names, opts);
  } catch {
    return null;
  }
}
"#;
    assert!(!has(&local("ts", normalised), SmellKind::DefensiveFallback));
}

#[test]
fn handlers_that_surface_the_error_are_not_broad() {
    let result = r#"
export function load(x, y) {
  try {
    const a = x ?? [];
    const b = y ?? {};
    return { ok: true, a, b };
  } catch (e) {
    return { ok: false, error: e };
  }
}
"#;
    assert!(!has(&local("ts", result), SmellKind::DefensiveFallback));

    let rethrown = r#"
export function load(x, y) {
  try {
    return use(x ?? [], y ?? {});
  } catch (e) {
    console.error(e);
    throw e;
  }
}
"#;
    assert!(!has(&local("ts", rethrown), SmellKind::DefensiveFallback));

    let python = r#"
def load(raw):
    try:
        return raw.get("names") or [], raw.get("options") or {}
    except:
        log("failed")
        raise
"#;
    assert!(!has(&local("py", python), SmellKind::DefensiveFallback));
}

#[test]
fn swallowing_handler_with_protected_fallbacks_fires() {
    let bare = r#"
export function load(x, y) {
  try {
    return use(x ?? [], y ?? {});
  } catch {
    return [];
  }
}
"#;
    assert!(has(&local("ts", bare), SmellKind::DefensiveFallback));

    let logged = r#"
export function load(x, y) {
  try {
    return use(x ?? [], y ?? {});
  } catch (e) {
    console.error("load failed", e);
    return [];
  }
}
"#;
    assert!(has(&local("ts", logged), SmellKind::DefensiveFallback));
}
