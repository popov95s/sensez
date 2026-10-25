//! Aggregated smells anchor at the first offending line and list every hit.

use super::detect_local;
use super::test_support as fixture;
use crate::config::smells::Smells;
use crate::report::{SmellFinding, SmellKind};

fn local(ext: &str, body: &str, cfg: &Smells) -> Vec<SmellFinding> {
    detect_local(&fixture::parsed(&format!("anchor.{ext}"), body), cfg)
}

fn only(findings: &[SmellFinding], kind: SmellKind) -> SmellFinding {
    let hits: Vec<_> = findings.iter().filter(|f| f.kind == kind).collect();
    assert_eq!(hits.len(), 1, "{findings:?}");
    hits[0].clone()
}

#[test]
fn nested_if_anchors_at_the_collapsible_outer_if() {
    let body = "\
function outer(a, b, c, x) {
  if (a) {
    x -= 1;
    if (b) {
      x += 1;
    }
  }
  if (b) {
    if (c) {
      return x;
    }
  }
  return 0;
}
";
    let finding = only(
        &local("ts", body, &Smells::default()),
        SmellKind::UnnecessaryNestedIf,
    );
    assert_eq!(finding.line, 8);
    assert_eq!(finding.lines, vec![8]);
    assert_eq!(finding.end_line, 8, "only the hit line is in scope");
    assert!(finding.message.contains("L8"), "{}", finding.message);
}

#[test]
fn nested_if_skips_outer_ifs_with_other_work() {
    let body = "\
function nestedIf(a, b, count) {
  if (a) { count += 1; if (b) { return count; } }
  return 0;
}
function nestedIfElse(a, b, count) {
  if (a) { return 1; } else { count += 1; if (b) { return count; } }
  return 0;
}
";
    let findings = local("ts", body, &Smells::default());
    assert!(!findings
        .iter()
        .any(|f| f.kind == SmellKind::UnnecessaryNestedIf));
}

#[test]
fn two_chains_give_one_finding_listing_both_lines() {
    let cfg = Smells {
        max_chain_depth: 3,
        ..Smells::default()
    };
    let body = "\
function read(order) {
  const a = order.customer.billing.address.city;
  const b = 1;
  return a + order.customer.billing.address.zip + b;
}
";
    let finding = only(&local("ts", body, &cfg), SmellKind::MessageChain);
    assert_eq!(finding.line, 2);
    assert_eq!(finding.lines, vec![2, 4]);
    assert_eq!(finding.metric, 4);
}

#[test]
fn python_chain_and_membership_anchor_at_their_lines() {
    let cfg = Smells {
        max_chain_depth: 3,
        ..Smells::default()
    };
    let body = "\
def route(invoice, kind):
    total = 1
    if kind in [\"admin\", \"owner\"]:
        return invoice.customer.profile.address.country
    return total
";
    let findings = local("py", body, &cfg);
    let chain = only(&findings, SmellKind::MessageChain);
    assert_eq!((chain.line, chain.lines.clone()), (4, vec![4]));
    let membership = only(&findings, SmellKind::LiteralMembership);
    assert_eq!((membership.line, membership.lines), (3, vec![3]));
}

#[test]
fn implicit_schema_anchors_at_the_first_keyed_access() {
    let body = "\
function load(cfg) {
  const port = 1;
  const host = cfg[\"host\"];
  return [host, cfg[\"user\"], cfg[\"pass\"], cfg[\"db\"], port];
}
";
    let finding = only(
        &local("js", body, &Smells::default()),
        SmellKind::ImplicitSchema,
    );
    assert_eq!(finding.line, 3);
}
