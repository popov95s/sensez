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
fn relative_resolves_to_sibling_key() {
    let root = Path::new("/proj");
    let importer = PathBuf::from("/proj/src/api/handlers.ts");
    assert_eq!(
        resolve_target(&ctx("./util"), &importer, root),
        "src/api/util"
    );
    assert_eq!(
        resolve_target(&ctx("../models"), &importer, root),
        "src/models"
    );
    assert_eq!(
        resolve_target(&ctx("../db/index"), &importer, root),
        "src/db"
    );
}

#[test]
fn relative_resolves_under_dot_prefixed_package_root() {
    let root = Path::new("./editors/ext");
    let importer = PathBuf::from("./editors/ext/src/main.ts");
    assert_eq!(resolve_target(&ctx("./util"), &importer, root), "src/util");
}

#[test]
fn bare_specifier_is_external() {
    let root = Path::new("/proj");
    let importer = PathBuf::from("/proj/src/a.ts");
    assert_eq!(resolve_target(&ctx("react"), &importer, root), "react");
    assert_eq!(
        resolve_target(&ctx("@scope/pkg"), &importer, root),
        "@scope/pkg"
    );
}

#[test]
fn containing_package_rules() {
    assert_eq!(containing_package("src/api/util", false), "src/api");
    assert_eq!(containing_package("src/api", true), "src/api");
    assert_eq!(containing_package("main", false), "");
}
