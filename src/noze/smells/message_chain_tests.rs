//! `message_chain` roots: namespace imports, ignored roots, and real navigation.

use super::detect_local;
use super::test_support as fixture;
use crate::config::smells::Smells;
use crate::report::SmellKind;

fn chains(ext: &str, body: &str, cfg: &Smells) -> Vec<usize> {
    detect_local(&fixture::parsed(&format!("chain.{ext}"), body), cfg)
        .into_iter()
        .filter(|f| f.kind == SmellKind::MessageChain)
        .map(|f| f.metric as usize)
        .collect()
}

#[test]
fn namespace_import_hop_is_not_navigation() {
    let body = "\
import * as ns from './api';
export function list() { return ns.admin.billing.invoices.list; }
";
    assert!(chains("ts", body, &Smells::default()).is_empty());
}

#[test]
fn ignored_roots_are_skipped() {
    let body = "\
const api = { admin: { billing: { invoices: { controller: { list: 1 } } } } };
export function list() { return api.admin.billing.invoices.controller.list; }
";
    assert_eq!(chains("ts", body, &Smells::default()), vec![5]);
    let cfg = Smells {
        message_chain_ignore_roots: vec!["api".to_string()],
        ..Smells::default()
    };
    assert!(chains("ts", body, &cfg).is_empty());
}

#[test]
fn deep_navigation_keeps_firing() {
    let ts = "\
export class Invoice {
  plan() { return this.owner.account.settings.billing.plan; }
}
";
    assert_eq!(chains("ts", ts, &Smells::default()), vec![5]);
    let py = "def f(a):\n    return a.b.c.d.e.f\n";
    assert_eq!(chains("py", py, &Smells::default()), vec![5]);
}

#[test]
fn named_imports_from_generated_modules_are_api_navigation() {
    let generated = "\
import { client } from './generated/client';
export function list() { return client.admin.billing.invoices.items.list; }
";
    assert!(chains("ts", generated, &Smells::default()).is_empty());
    let handwritten = generated.replace("./generated/client", "./client");
    assert_eq!(chains("ts", &handwritten, &Smells::default()), vec![5]);
}
