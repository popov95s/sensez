use super::detect_local;
use crate::config::smells::Smells;
use crate::report::{SmellFinding, SmellKind};
use crate::spine::parser::parse_file;
use std::fs;

fn local(ext: &str, source: &str) -> Vec<SmellFinding> {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join(format!("sample.{ext}"));
    fs::write(&path, source).unwrap();
    let file = parse_file(&path, 0).unwrap();
    detect_local(&file, &review_risk_config())
}

fn review_risk_config() -> Smells {
    let mut config = Smells::default();
    config.disabled.retain(|kind| {
        *kind != SmellKind::DefensiveFallback && *kind != SmellKind::RedundantValidation
    });
    config
}

fn has(findings: &[SmellFinding], kind: SmellKind) -> bool {
    findings.iter().any(|finding| finding.kind == kind)
}

#[test]
fn fallback_soup_requires_broad_handler_and_multiple_defaults() {
    let bad = r#"
def load(raw):
    try:
        names = raw.get("names") or []
        options = raw.get("options") or {}
        return names, options
    except:
        return [], {}
"#;
    assert!(has(&local("py", bad), SmellKind::DefensiveFallback));

    let valid = r#"
def load(raw):
    try:
        return parse_payload(raw)
    except InvalidPayload as error:
        raise UserInputError() from error
"#;
    assert!(!has(&local("py", valid), SmellKind::DefensiveFallback));
}

#[test]
fn fallback_detection_uses_syntax_not_source_fragments() {
    let source = r#"
def load(raw):
    try:
        note = "examples: value or [], value or {}"
        return raw, note
    except:
        return None
"#;
    assert!(!has(&local("py", source), SmellKind::DefensiveFallback));
}

#[test]
fn javascript_fallback_detection_uses_operator_and_literal_nodes() {
    let source = r#"
export function load(raw) {
  try {
    const names = raw.names || [];
    const options = raw.options ?? {};
    return { names, options };
  } catch {
    return null;
  }
}
"#;
    assert!(has(&local("js", source), SmellKind::DefensiveFallback));
}

#[test]
fn repeated_guard_is_distinct_from_different_validations() {
    let bad = r#"
def save(value):
    if value is None:
        return
    prepare(value)
    if value is None:
        return
"#;
    assert!(has(&local("py", bad), SmellKind::RedundantValidation));

    let valid = r#"
def save(value):
    if value is None:
        return
    if not value.is_valid:
        return
"#;
    assert!(!has(&local("py", valid), SmellKind::RedundantValidation));
}

#[test]
fn two_implementations_with_divergent_surfaces_are_flagged() {
    let bad = r#"
from typing import Protocol

class Worker(Protocol):
    def run(self): ...

class FileWorker(Worker):
    def run(self): pass
    def open_file(self): pass
    def rotate_file(self): pass

class QueueWorker(Worker):
    def run(self): pass
    def reserve_job(self): pass
    def acknowledge_job(self): pass
"#;
    assert!(has(&local("py", bad), SmellKind::DivergentAbstraction));
}

#[test]
fn cohesive_two_implementation_protocol_is_not_flagged() {
    let valid = r#"
from typing import Protocol

class Encoder(Protocol):
    def encode(self): ...

class JsonEncoder(Encoder):
    def encode(self): pass
    def media_type(self): pass
    def content_type(self): pass

class XmlEncoder(Encoder):
    def encode(self): pass
    def media_type(self): pass
    def content_type(self): pass
"#;
    assert!(!has(&local("py", valid), SmellKind::DivergentAbstraction));
}
