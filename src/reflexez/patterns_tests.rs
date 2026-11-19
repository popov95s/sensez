use super::*;

#[test]
fn relative_patterns_lose_dot_components() {
    let root = Path::new("/repo");
    let importer = Path::new("/repo/src/routes/index.ts");

    let patterns = path_patterns(importer, "./pages/*.ts", root, &mut |_| None);
    assert_eq!(patterns, vec!["/repo/src/routes/pages/*.ts".to_string()]);

    let nested = path_patterns(importer, "./../shared/*.ts", root, &mut |_| None);
    assert!(nested[0].starts_with("/repo/src/shared/"));
}

#[test]
fn alias_patterns_use_resolved_path() {
    let root = Path::new("/repo");
    let importer = Path::new("/repo/src/registry.ts");
    let mut alias = |spec: &str| {
        spec.strip_prefix("@/")
            .map(|rest| PathBuf::from("/repo/src").join(rest))
    };

    let patterns = path_patterns(importer, "@/handlers/*.ts", root, &mut alias);

    assert_eq!(patterns, vec!["/repo/src/handlers/*.ts".to_string()]);
}
