//! `swallowed_error_sentinel`: handlers that hide failures behind placeholders.

use super::test_support as fixture;
use crate::config::smells::{Smells, Strictness};
use crate::report::{Severity, SmellFinding, SmellKind};

fn findings_at(name: &str, source: &str, strictness: Strictness) -> Vec<SmellFinding> {
    let config = Smells {
        swallowed_error_strictness: strictness,
        ..Smells::default()
    };
    fixture::local_of(name, source, &config, SmellKind::SwallowedErrorSentinel)
}

fn findings(name: &str, source: &str) -> Vec<SmellFinding> {
    findings_at(name, source, Strictness::Medium)
}

const TS_NULL: &str = r#"
export async function loadInvoice(id: string) {
  try {
    return await ledger.invoices.fetch(id);
  } catch (failure) {
    console.warn("invoice fetch failed", failure);
    return null;
  }
}
"#;

#[test]
fn ts_catch_returning_null_names_the_sentinel_line_and_fix() {
    let found = findings("billing.ts", TS_NULL);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].line, 5);
    assert_eq!(found[0].severity, Severity::Warning);
    assert!(found[0]
        .message
        .starts_with("catch at line 5 returns `null`"));
    assert!(found[0]
        .message
        .contains("rethrow it or return a typed failure"));
}

#[test]
fn python_broad_except_returning_none_uses_python_wording() {
    let source = "def load_invoice(key):\n    try:\n        return ledger.fetch(key)\n    except Exception:\n        return None\n";
    let found = findings("billing.py", source);
    assert_eq!(found.len(), 1);
    assert!(found[0]
        .message
        .starts_with("except at line 4 returns `None`"));
    assert!(found[0].message.contains("re-raise it"));
}

#[test]
fn python_narrow_except_is_quiet() {
    let source = "def lookup(table, key):\n    try:\n        return table[key]\n    except KeyError:\n        return None\n";
    assert!(findings("lookup.py", source).is_empty());
}

#[test]
fn typed_failure_reading_the_error_is_quiet() {
    let source = r#"
export async function loadInvoice(id: string) {
  try {
    return { ok: true, value: await ledger.invoices.fetch(id) };
  } catch (failure) {
    return { ok: false, error: failure };
  }
}
"#;
    assert!(findings("billing.ts", source).is_empty());
}

#[test]
fn rethrowing_handler_is_quiet() {
    let source = r#"
export function readConfig(text: string) {
  try {
    return decode(text);
  } catch (failure) {
    console.error("bad config", failure);
    throw failure;
  }
}
"#;
    assert!(findings("config.ts", source).is_empty());
}

#[test]
fn handler_that_notifies_the_user_is_quiet() {
    let source = r#"
export async function saveDraft(draft: Draft) {
  try {
    await drafts.save(draft);
    return true;
  } catch {
    toast.error("Could not save the draft");
    return false;
  }
}
"#;
    assert!(findings("draft.ts", source).is_empty());
}

#[test]
fn storage_probe_is_quiet() {
    let source = r#"
export function readTheme() {
  try {
    return window.localStorage.getItem("theme");
  } catch {
    return null;
  }
}
"#;
    assert!(findings("theme.ts", source).is_empty());
}

#[test]
fn try_named_function_with_nullable_return_is_quiet() {
    let source = r#"
export function tryDecodeBadge(raw: string): Badge | null {
  try {
    return decodeBadge(raw);
  } catch {
    return null;
  }
}
"#;
    assert!(findings("badge.ts", source).is_empty());
}

#[test]
fn predicate_answering_false_is_quiet() {
    let source = r#"
export function isSecureLink(raw: string) {
  try {
    return new URL(raw).protocol === "https:";
  } catch {
    return false;
  }
}
"#;
    assert!(findings("links.ts", source).is_empty());
}

const SINGLE_PARSE: &str = r#"
export function readSettings(text: string) {
  try {
    return JSON.parse(text);
  } catch {
    return undefined;
  }
}
"#;

#[test]
fn single_json_parse_is_reported_at_medium() {
    assert_eq!(findings("settings.ts", SINGLE_PARSE).len(), 1);
}

#[test]
fn single_json_parse_is_exempt_at_low() {
    assert!(findings_at("settings.ts", SINGLE_PARSE, Strictness::Low).is_empty());
}

const LOG_ONLY: &str = r#"
export async function syncBadges() {
  try {
    await badges.sync();
  } catch (failure) {
    logger.warn("badge sync failed", failure);
  }
}
"#;

#[test]
fn log_only_handler_is_quiet_at_medium() {
    assert!(findings("badges.ts", LOG_ONLY).is_empty());
}

#[test]
fn log_only_handler_is_reported_at_high() {
    let found = findings_at("badges.ts", LOG_ONLY, Strictness::High);
    assert_eq!(found.len(), 1);
    assert!(found[0].message.contains("only logs or ignores the error"));
}

#[test]
fn log_only_handler_inside_a_loop_is_quiet_at_high() {
    let source = r#"
export async function syncAll(ids: string[]) {
  for (const id of ids) {
    try {
      await badges.sync(id);
    } catch (failure) {
      logger.warn("badge sync failed", failure);
    }
  }
}
"#;
    assert!(findings_at("badges.ts", source, Strictness::High).is_empty());
}

const DEFAULT_LITERAL: &str = "def plan_name(account):\n    try:\n        return account.plan.title\n    except Exception:\n        return \"unknown\"\n";

#[test]
fn default_literal_is_quiet_at_medium() {
    assert!(findings("plans.py", DEFAULT_LITERAL).is_empty());
}

#[test]
fn default_literal_is_reported_at_high() {
    assert_eq!(
        findings_at("plans.py", DEFAULT_LITERAL, Strictness::High).len(),
        1
    );
}

#[test]
fn cleanup_inside_another_handler_is_quiet() {
    let source = r#"
export async function publish(post: Post) {
  try {
    await feed.publish(post);
  } catch (failure) {
    try {
      await feed.markFailed(post.id);
    } catch {
      return null;
    }
    throw failure;
  }
}
"#;
    assert!(findings("feed.ts", source).is_empty());
}

#[test]
fn test_sources_are_quiet() {
    assert!(findings("billing.test.ts", TS_NULL).is_empty());
}

#[test]
fn python_except_binding_the_error_is_reported() {
    let source = "def load_invoice(key):\n    try:\n        return ledger.fetch(key)\n    except Exception as failure:\n        log.warning(\"fetch failed: %s\", failure)\n        return None\n";
    assert_eq!(findings("billing.py", source).len(), 1);
}

#[test]
fn python_except_returning_the_bound_error_is_quiet() {
    let source = "def load_invoice(key):\n    try:\n        return ledger.fetch(key)\n    except Exception as failure:\n        return Failure(failure)\n";
    assert!(findings("billing.py", source).is_empty());
}
