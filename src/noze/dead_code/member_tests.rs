use crate::config::model::Config;
use crate::noze;
use crate::report::Confidence;
use crate::spine::parser::{parse_file, SymbolKind};
use std::fs;

#[test]
fn unused_method_survives_report_filter_when_cross_file_evidence_is_clean() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    fs::create_dir_all(dir.join("pkg")).unwrap();
    fs::write(dir.join("pkg/__init__.py"), "").unwrap();
    fs::write(
        dir.join("pkg/widget.py"),
        "class Widget:\n    def render(self):\n        return self.size()\n    def size(self):\n        return 1\n    def orphan(self):\n        return 2\n",
    )
    .unwrap();
    fs::write(
        dir.join("pkg/app.py"),
        "from pkg.widget import Widget\n\n\
         def main():\n    w = Widget()\n    return w.render()\n",
    )
    .unwrap();

    let files = parse_files(&dir, &["pkg/__init__.py", "pkg/widget.py", "pkg/app.py"]);
    let report = run_with_unused_methods(&files);
    let methods: Vec<_> = report
        .dead_code
        .iter()
        .filter(|finding| finding.kind == SymbolKind::Method)
        .map(|finding| (finding.symbol.as_str(), finding.confidence))
        .collect();

    assert!(methods.contains(&("orphan", Confidence::High)));
    assert!(!methods.iter().any(|(symbol, _)| *symbol == "render"));
}

#[test]
fn unused_property_survives_report_filter_with_high_confidence() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    fs::write(
        dir.join("model.py"),
        "class User:\n    name: str\n    stale: str\n\n    def label(self):\n        return self.name\n",
    )
    .unwrap();

    let files = parse_files(&dir, &["model.py"]);
    let graph = crate::spine::graph::build(&files, &[]);
    let mut config = Config::default();
    config.dead_code.unused_properties = true;

    let report = noze::run(&files, &graph, &config);
    let stale = report
        .dead_code
        .iter()
        .find(|finding| finding.symbol == "User.stale")
        .expect("stale property should be reported");

    assert_eq!(stale.kind, SymbolKind::Property);
    assert_eq!(stale.confidence, Confidence::High);
}

#[test]
fn overloaded_method_declarations_report_once() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    fs::write(
        dir.join("service.py"),
        "from typing import overload\n\nclass Service:\n    value: int\n\n    def touch(self):\n        return self.value\n\n    @overload\n    def orphan(self, value: str) -> str: ...\n\n    @overload\n    def orphan(self, value: int) -> int: ...\n\n    def orphan(self, value):\n        return value\n",
    )
    .unwrap();

    let files = parse_files(&dir, &["service.py"]);
    let report = run_with_unused_methods(&files);
    let count = report
        .dead_code
        .iter()
        .filter(|finding| finding.kind == SymbolKind::Method && finding.symbol == "orphan")
        .count();

    assert_eq!(count, 1);
}

#[test]
fn methods_overriding_known_base_methods_are_not_reported() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    fs::write(
        dir.join("service.py"),
        "class Base:\n    def hook(self):\n        return 1\n\nclass Child(Base):\n    value: int\n\n    def touch(self):\n        return self.value\n\n    def hook(self):\n        return 2\n\n    def orphan(self):\n        return 3\n",
    )
    .unwrap();

    let files = parse_files(&dir, &["service.py"]);
    let report = run_with_unused_methods(&files);
    let methods: Vec<_> = report
        .dead_code
        .iter()
        .filter(|finding| finding.kind == SymbolKind::Method)
        .map(|finding| finding.symbol.as_str())
        .collect();

    assert!(!methods.contains(&"hook"));
    assert!(methods.contains(&"orphan"));
}

#[test]
fn methods_on_package_reexported_classes_are_external_api() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    fs::create_dir_all(dir.join("pkg")).unwrap();
    fs::write(
        dir.join("pkg/__init__.py"),
        "from .widget import Widget as Widget\n",
    )
    .unwrap();
    fs::write(
        dir.join("pkg/widget.py"),
        "class Widget:\n    name: str\n\n    def touch(self):\n        return self.name\n\n    def external_api(self):\n        return 1\n",
    )
    .unwrap();

    let files = parse_files(&dir, &["pkg/__init__.py", "pkg/widget.py"]);
    let report = run_with_unused_methods(&files);

    assert!(!report
        .dead_code
        .iter()
        .any(|finding| finding.kind == SymbolKind::Method && finding.symbol == "external_api"));
}

fn parse_files(dir: &std::path::Path, names: &[&str]) -> Vec<crate::spine::parser::ParsedFile> {
    names
        .iter()
        .enumerate()
        .map(|(index, name)| parse_file(&dir.join(name), index as u32).unwrap())
        .collect()
}

fn run_with_unused_methods(
    files: &[crate::spine::parser::ParsedFile],
) -> crate::report::AnalysisReport {
    let graph = crate::spine::graph::build(files, &[]);
    let mut config = Config::default();
    config.dead_code.unused_methods = true;
    noze::run(files, &graph, &config)
}
