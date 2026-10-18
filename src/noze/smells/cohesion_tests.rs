use super::detect_local;
use crate::config::smells::Smells;
use crate::report::SmellKind;
use crate::spine::parser::parse_file;
use std::fs;

#[test]
fn wrapper_calls_connect_cohesion_clusters() {
    let source = r#"
class Cache:
    def get(self, key):
        return self._read(key)

    def contains(self, key):
        return self._read(key) is not None

    def _read(self, key):
        return self.storage.get(key)
"#;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("sample.py");
    fs::write(&path, source).unwrap();
    let file = parse_file(&path, 0).unwrap();
    let findings = detect_local(&file, &Smells::default());
    assert!(
        !findings
            .iter()
            .any(|finding| finding.kind == SmellKind::DivergentChange),
        "{findings:?}"
    );
}

fn divergent_ts(source: &str) -> bool {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("sample.ts");
    fs::write(&path, source).unwrap();
    let file = parse_file(&path, 0).unwrap();
    detect_local(&file, &Smells::default())
        .iter()
        .any(|finding| finding.kind == SmellKind::DivergentChange)
}

#[test]
fn interface_implementations_and_no_op_methods_are_not_divergent() {
    assert!(!divergent_ts(
        "export class FakeObs implements Observer {\n  observe() { this.a.push(1); }\n  unobserve() { this.b = 1; }\n  disconnect() { this.c = 0; }\n}\n"
    ));
    assert!(!divergent_ts(
        "export class Queue {\n  push(x) { this.items.push(x); }\n  pop() { return this.items.pop(); }\n  reset() {}\n  close() { return; }\n  flush() { throw new Error(\"not implemented\"); }\n}\n"
    ));
}

#[test]
fn one_line_methods_touching_fields_still_count() {
    assert!(divergent_ts(
        "export class FakeObs {\n  observe() { this.a.push(1); }\n  unobserve() { this.b = 1; }\n  disconnect() { this.c = 0; }\n}\n"
    ));
}

#[test]
fn disjoint_multi_statement_method_groups_are_divergent() {
    assert!(divergent_ts(
        "export class Editor {\n  renderFrame() {\n    this.canvas.clear();\n    this.canvas.draw();\n  }\n  renderCursor() {\n    this.canvas.move();\n    this.canvas.blink();\n  }\n  saveDoc() {\n    this.db.open();\n    this.db.write();\n  }\n  saveMeta() {\n    this.db.open();\n    this.db.flush();\n  }\n}\n"
    ));
}
