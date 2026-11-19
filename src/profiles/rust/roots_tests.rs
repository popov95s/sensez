use super::*;

#[test]
fn module_key_strips_ext_and_collapses_mod_and_lib() {
    let root = Path::new("/proj");
    assert_eq!(
        module_name(Path::new("/proj/src/noze/mod.rs"), root),
        "src/noze"
    );
    assert_eq!(
        module_name(Path::new("/proj/src/noze.rs"), root),
        "src/noze"
    );
    assert_eq!(module_name(Path::new("/proj/src/lib.rs"), root), "src");
    assert_eq!(
        module_name(Path::new("/proj/src/main.rs"), root),
        "src/main"
    );
}
