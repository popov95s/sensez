use super::*;

#[test]
fn resolves_importlib_constant_and_pattern() {
    let source = br#"name = "pkg.feature"
importlib.import_module(name)
importlib.import_module(f"pkg.pages.{page}")
"#;
    let facts = scan(source, "sample").unwrap();
    assert_eq!(facts.imports[0].target_module, "pkg.feature");
    assert_eq!(facts.patterns, vec![vec!["pkg.pages.*"]]);
    assert_eq!(facts.unresolved, 0);
}
