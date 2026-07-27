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
