use super::*;
use std::fs;

#[test]
fn detects_generated_codecs_by_header_not_name_alone() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("encodings");
    fs::create_dir_all(&dir).unwrap();
    let codec = dir.join("cp037.py");
    let handwritten = dir.join("aliases.py");
    fs::write(
        &codec,
        "\"\"\" Python Character Mapping Codec cp037 generated from 'x' with gencodec.py.\n\"\"\"\n",
    )
    .unwrap();
    fs::write(&handwritten, "# Python Character Mapping Codec\n").unwrap();

    assert!(is_generated_or_data_source(&codec));
    assert!(!is_generated_or_data_source(&handwritten));
}

#[test]
fn detects_stainless_openapi_generated_python_modules() {
    let tmp = tempfile::tempdir().unwrap();
    let generated = tmp.path().join("model.py");
    let handwritten = tmp.path().join("handwritten.py");
    fs::write(
        &generated,
        "# File generated from our OpenAPI spec by Stainless. See CONTRIBUTING.md for details.\n\nclass Model:\n    pass\n",
    )
    .unwrap();
    fs::write(
        &handwritten,
        "# Talks about Stainless and OpenAPI, but is handwritten.\nclass Model:\n    pass\n",
    )
    .unwrap();

    assert!(is_generated_or_data_source(&generated));
    assert!(!is_generated_or_data_source(&handwritten));
}

#[test]
fn detects_schema_validators_and_locale_format_data() {
    assert!(is_generated_or_data_source(Path::new(
        "pkg/fastjsonschema_validations.py"
    )));
    assert!(is_generated_or_data_source(Path::new(
        "django/conf/locale/fr/formats.py"
    )));
    assert!(!is_generated_or_data_source(Path::new(
        "pkg/locale_helpers/formats.py"
    )));
}

#[test]
fn path_rules_are_composable() {
    let rule = PathRule {
        extension: Some("py"),
        file_name: Some("data.py"),
        parent: None,
        any_components: &[&["generated", "autogen"], &["schemas"]],
        excluded_stems: &[],
    };

    assert!(rule.matches(Path::new("pkg/generated/schemas/data.py")));
    assert!(rule.matches(Path::new("pkg/autogen/schemas/data.py")));
    assert!(!rule.matches(Path::new("pkg/generated/other/data.py")));
    assert!(!rule.matches(Path::new("pkg/generated/schemas/data.ts")));
}
