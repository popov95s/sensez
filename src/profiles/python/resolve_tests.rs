use super::*;

fn ctx(target: &str) -> ImportContext {
    ImportContext {
        source_module: "x".into(),
        target_module: target.into(),
        imported_symbols: vec![],
        bindings: vec![],
        binding_phases: vec![],
        line: 1,
        column: 1,
        phase: crate::spine::ir::ImportPhase::Runtime,
        is_inline: false,
        is_module_decl: false,
        is_reexport: false,
        enclosing_scope: None,
    }
}

#[test]
fn absolute_passthrough() {
    assert_eq!(resolve_target(&ctx("a.b.c"), "pkg.sub"), "a.b.c");
}

#[test]
fn relative_levels() {
    assert_eq!(resolve_target(&ctx("."), "pkg.sub"), "pkg.sub");
    assert_eq!(resolve_target(&ctx(".mod"), "pkg.sub"), "pkg.sub.mod");
    assert_eq!(resolve_target(&ctx("..other"), "pkg.sub"), "pkg.other");
    assert_eq!(resolve_target(&ctx(".."), "pkg.sub"), "pkg");
}

#[test]
fn containing_package_rules() {
    assert_eq!(containing_package("pkg.sub.mod", false), "pkg.sub");
    assert_eq!(containing_package("pkg.sub", true), "pkg.sub");
    assert_eq!(containing_package("top", false), "");
}
