use super::*;

#[test]
fn resolves_constant_and_pattern_dynamic_imports() {
    let source = br#"const target = './feature'; import(target); import(`./pages/${name}.ts`);"#;
    let facts = scan(Path::new("sample.ts"), source, "sample").unwrap();
    assert_eq!(facts.imports[0].target_module, "./feature");
    assert_eq!(facts.patterns, vec![vec!["./pages/*.ts"]]);
    assert_eq!(facts.unresolved, 0);
}

#[test]
fn resolves_array_glob_with_negation() {
    let source = br#"import.meta.glob(["./handlers/*.ts", "!./handlers/skip.ts"]);"#;
    let facts = scan(Path::new("registry.ts"), source, "registry").unwrap();
    assert_eq!(
        facts.patterns,
        vec![vec!["./handlers/*.ts", "!./handlers/skip.ts"]]
    );
    assert_eq!(facts.unresolved, 0);
}
