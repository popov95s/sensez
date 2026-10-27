use super::detect_local;
use super::test_support as fixture;
use crate::config::smells::Smells;
use crate::report::{SmellFinding, SmellKind};
use crate::spine::parser::parse_file;

pub(super) fn local(ext: &str, source: &str) -> Vec<SmellFinding> {
    local_with_config(ext, source, &review_risk_config())
}

pub(super) fn local_with_config(ext: &str, source: &str, config: &Smells) -> Vec<SmellFinding> {
    let file = fixture::parsed(&format!("sample.{ext}"), source);
    detect_local(&file, config)
}

pub(super) fn review_risk_config() -> Smells {
    let mut config = Smells::default();
    config.disabled.retain(|kind| {
        *kind != SmellKind::DefensiveFallback && *kind != SmellKind::RedundantValidation
    });
    config
}

pub(super) fn has(findings: &[SmellFinding], kind: SmellKind) -> bool {
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

#[path = "review_risks_wrapper_tests.rs"]
mod wrapper_tests;

#[test]
fn tests_without_behavioral_checks_are_reported() {
    let mut config = Smells::default();
    config
        .disabled
        .retain(|kind| *kind != SmellKind::WeakTestOracle);
    for (name, source) in [
        (
            "test_example.py",
            "def test_saves():\n    store.save(42)\n    store.flush()\n",
        ),
        (
            "example.test.js",
            "test('saves', () => {\n  store.save(42);\n  store.flush();\n});\n",
        ),
        (
            "example.test.ts",
            "test('saves', () => {\n  store.save(42);\n  store.flush();\n});\n",
        ),
    ] {
        let file = fixture::parsed(name, source);
        let findings = detect_local(&file, &config);
        assert_eq!(
            findings
                .iter()
                .filter(|f| f.kind == SmellKind::WeakTestOracle)
                .count(),
            1,
            "{name}"
        );
    }
}

#[test]
fn documented_python_test_example_is_a_test_case() {
    let source = include_str!("../../../docs/examples/smells/weak_test_oracle/test_example.py");
    let file = fixture::parsed("test_example.py", source);
    let test = file
        .walked
        .units
        .functions
        .iter()
        .find(|unit| unit.name == "test_saves")
        .unwrap();
    assert!(test.review_risks.is_test_case, "{test:?}");
    assert_eq!(test.review_risks.test_checks, 0);
    let mut config = Smells::default();
    config
        .disabled
        .retain(|kind| *kind != SmellKind::WeakTestOracle);
    assert!(detect_local(&file, &config)
        .iter()
        .any(|finding| finding.kind == SmellKind::WeakTestOracle));
}

fn opt_in_oracle_findings(toml: &str) -> Vec<SmellFinding> {
    use crate::config::Config;
    use crate::noze::source_scope::SourceScope;
    use crate::spine::graph::CodebaseGraph;

    let temp = tempfile::tempdir().unwrap();
    let source = include_str!("../../../docs/examples/smells/weak_test_oracle/test_example.py");
    let path = crate::test_support::write(temp.path(), "test_example.py", source);
    crate::test_support::write(temp.path(), "sensez.toml", toml);
    let config = Config::load(temp.path()).unwrap();
    super::detect(
        &[parse_file(&path, 0).unwrap()],
        &CodebaseGraph::default(),
        &config.smells,
        &SourceScope::for_smells(&config.smells),
    )
}

#[test]
fn opt_in_test_rule_inspects_baseline_excluded_tests() {
    let findings = opt_in_oracle_findings("[smells.python]\ndisabled = []\n");
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert_eq!(findings[0].kind, SmellKind::WeakTestOracle);
}

#[test]
fn opt_in_test_rule_honors_user_exclusions() {
    let findings = opt_in_oracle_findings(
        "[smells]\nexclude = [\"**/test_example.py\"]\n[smells.python]\ndisabled = []\n",
    );
    assert!(findings.is_empty(), "{findings:?}");
}

#[test]
fn assertions_and_expected_exceptions_are_oracles() {
    let mut config = Smells::default();
    config
        .disabled
        .retain(|kind| *kind != SmellKind::WeakTestOracle);
    for (name, source) in [
        (
            "test_example.py",
            "def test_saves():\n    store.save(42)\n    assert store.count == 1\n",
        ),
        (
            "test_raises.py",
            "def test_invalid():\n    with pytest.raises(ValueError):\n        parse('bad')\n",
        ),
        (
            "example.test.ts",
            "test('saves', () => {\n  store.save(42);\n  expect(store.count).toBe(1);\n});\n",
        ),
        (
            "example.spec.js",
            "test.skip('later', () => {\n  store.save(42);\n});\n",
        ),
        (
            "test_example.ts",
            "test('not a JS/TS test file', () => {\n  store.save(42);\n});\n",
        ),
        (
            "example.test.py",
            "def test_not_a_python_test_file():\n    store.save(42)\n",
        ),
    ] {
        let file = fixture::parsed(name, source);
        assert!(
            !detect_local(&file, &config)
                .iter()
                .any(|f| f.kind == SmellKind::WeakTestOracle),
            "{name}"
        );
    }
}

#[test]
fn profile_assertion_calls_are_recognized_by_the_shared_matcher() {
    let mut config = Smells::default();
    config
        .disabled
        .retain(|kind| *kind != SmellKind::WeakTestOracle);
    for (name, source) in [
        (
            "test_checks.py",
            "def test_checks():\n    assert_equal(save(1), 1)\n",
        ),
        (
            "example.test.ts",
            "test('checks', () => {\n  assert.equal(save(1), 1);\n});\n",
        ),
        (
            "example.spec.ts",
            "test('snapshot', () => {\n  expect(save(1)).matchSnapshot();\n});\n",
        ),
    ] {
        let file = fixture::parsed(name, source);
        assert!(
            !detect_local(&file, &config)
                .iter()
                .any(|finding| finding.kind == SmellKind::WeakTestOracle),
            "{name}"
        );
    }
}
