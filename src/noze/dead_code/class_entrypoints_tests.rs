use super::*;
use crate::spine::parser::parse_file;
use std::fs;

#[test]
fn subclasses_of_configured_bases_are_entrypoints() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("apps.py");
    fs::write(
        &file,
        "from framework import AppConfig\n\n\
         class SimpleAdminConfig(AppConfig):\n    pass\n\n\
         class AdminConfig(SimpleAdminConfig):\n    pass\n\n\
         class Plain:\n    pass\n",
    )
    .unwrap();
    let parsed = parse_file(&file, 0).unwrap();
    let entrypoints = ClassEntrypoints::from_files(&[parsed], &["AppConfig".to_string()]);

    assert!(entrypoints.is_entrypoint(&file, "AdminConfig", SymbolKind::Class));
    assert!(entrypoints.is_entrypoint(&file, "SimpleAdminConfig", SymbolKind::Class));
    assert!(!entrypoints.is_entrypoint(&file, "Plain", SymbolKind::Class));
}

#[test]
fn dotted_bases_match_by_short_or_full_name() {
    let roots = HashSet::from(["framework.PluginBase"]);
    let mut class_bases = HashMap::new();
    class_bases.insert("Plugin", vec!["framework.PluginBase".to_string()]);

    assert!(dynamic_classes(&class_bases, &roots).contains("Plugin"));

    let roots = HashSet::from(["PluginBase"]);
    assert!(dynamic_classes(&class_bases, &roots).contains("Plugin"));
}
