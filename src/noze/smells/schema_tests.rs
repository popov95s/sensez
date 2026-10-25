use super::detect_local;
use super::test_support as fixture;
use crate::config::smells::Smells;
use crate::report::{SmellFinding, SmellKind};

fn local(source: &str) -> Vec<SmellFinding> {
    let file = fixture::parsed("sample.py", source);
    detect_local(&file, &Smells::default())
}

fn has(findings: &[SmellFinding], kind: SmellKind) -> bool {
    findings.iter().any(|finding| finding.kind == kind)
}

#[test]
fn schema_keys_propagate_across_helpers() {
    let source = r#"
def identity(data):
    return data["id"], data["name"]

def routing(data):
    return data["queue"], data["region"]

def handle(payload):
    identity(payload)
    routing(payload)
"#;
    assert!(has(&local(source), SmellKind::ImplicitSchema));
}

#[test]
fn typed_validator_consumption_is_not_loose_typing() {
    let source = r#"
from typing import Any

def decode(payload: dict[str, Any]) -> User:
    return User.model_validate(payload)
"#;
    let findings = local(source);
    assert!(!has(&findings, SmellKind::LooseTyping), "{findings:?}");
    assert!(!has(&findings, SmellKind::ImplicitSchema), "{findings:?}");
}

#[test]
fn typed_json_boundary_is_not_an_implicit_domain_schema() {
    let source = r#"
def parse_user(payload: dict[str, object]) -> User:
    return User(
        payload["id"],
        payload["name"],
        payload["email"],
        payload["team"],
    )
"#;
    let findings = local(source);
    assert!(!has(&findings, SmellKind::ImplicitSchema), "{findings:?}");
}

fn local_ts(source: &str) -> Vec<SmellFinding> {
    let file = fixture::parsed("sample.ts", source);
    detect_local(&file, &Smells::default())
}

#[test]
fn data_keyed_and_open_maps_are_not_implicit_schemas() {
    for source in [
        "export function tally(rows: Row[]) {\n  const counts: Record<string, number> = {};\n  counts[\"alpha\"] = 0;\n  counts[\"beta\"] = 0;\n  counts[\"gamma\"] = 0;\n  counts[\"delta\"] = 0;\n  for (const r of rows) counts[r.cat] += 1;\n  return counts;\n}\n",
        "export function sum(m: Record<\"a\" | \"b\" | \"c\" | \"d\", number>) {\n  return m[\"a\"] + m[\"b\"] + m[\"c\"] + m[\"d\"];\n}\n",
        "export function label(attrs: { [k: string]: string }) {\n  return attrs[\"id\"] + attrs[\"role\"] + attrs[\"title\"] + attrs[\"lang\"];\n}\n",
    ] {
        let findings = local_ts(source);
        assert!(
            !has(&findings, SmellKind::ImplicitSchema),
            "{source}\n{findings:?}"
        );
    }
}

#[test]
fn loose_or_unannotated_string_keyed_receivers_stay_implicit_schemas() {
    for source in [
        "export function f(cfg: any) {\n  return cfg[\"host\"] + cfg[\"port\"] + cfg[\"user\"] + cfg[\"pass\"];\n}\n",
        "export function f(cfg) {\n  return cfg[\"host\"] + cfg[\"port\"] + cfg[\"user\"] + cfg[\"pass\"];\n}\n",
    ] {
        let findings = local_ts(source);
        assert!(
            has(&findings, SmellKind::ImplicitSchema),
            "{source}\n{findings:?}"
        );
    }
}
