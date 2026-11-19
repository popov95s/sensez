use super::*;

#[test]
fn module_key_strips_ext_and_index() {
    let root = Path::new("/proj");
    assert_eq!(
        module_name(Path::new("/proj/src/util.ts"), root),
        "src/util"
    );
    assert_eq!(
        module_name(Path::new("/proj/src/api/index.js"), root),
        "src/api"
    );
    assert_eq!(module_name(Path::new("/proj/main.jsx"), root), "main");
}

#[test]
fn path_key_strips_ext_and_index() {
    assert_eq!(
        path_key(Path::new("/proj/src/../lib/index.ts")),
        PathBuf::from("/proj/lib")
    );
    assert_eq!(
        path_key(Path::new("./src/util.test.ts")),
        PathBuf::from("src/util.test")
    );
}

#[test]
fn normalize_resolves_dot_dot() {
    assert_eq!(
        normalize(Path::new("/proj/src/a/../b/./c")),
        PathBuf::from("/proj/src/b/c")
    );
}
