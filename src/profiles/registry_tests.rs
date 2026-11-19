use super::{is_test_source, Language};
use std::path::Path;

#[test]
fn test_sources_follow_the_owning_profiles() {
    #[cfg(feature = "lang-python")]
    {
        assert!(is_test_source(
            Language::Python,
            Path::new("tests/test_api.py")
        ));
        assert!(is_test_source(
            Language::Python,
            Path::new("src/api_test.py")
        ));
        assert!(!is_test_source(
            Language::Python,
            Path::new("src/api.test.py")
        ));
    }
    #[cfg(feature = "lang-javascript")]
    {
        assert!(is_test_source(
            Language::JavaScript,
            Path::new("src/api.test.js")
        ));
        assert!(is_test_source(
            Language::JavaScript,
            Path::new("src/api.spec.jsx")
        ));
    }
    #[cfg(feature = "lang-typescript")]
    {
        assert!(is_test_source(
            Language::TypeScript,
            Path::new("src/api.spec.tsx")
        ));
        assert!(is_test_source(
            Language::TypeScript,
            Path::new("__tests__/api.ts")
        ));
        assert!(is_test_source(
            Language::TypeScript,
            Path::new("src/api.test.mts")
        ));
        assert!(is_test_source(
            Language::TypeScript,
            Path::new("src/api.spec.cts")
        ));
        assert!(!is_test_source(
            Language::TypeScript,
            Path::new("src/test_api.ts")
        ));
    }
}

#[cfg(feature = "lang-typescript")]
#[test]
fn test_support_layouts_are_test_code_but_not_test_sources() {
    use super::is_test_or_support_source;
    for path in [
        "src/__mocks__/api.ts",
        "src/__fixtures__/user.ts",
        "e2e/login.ts",
        "src/test-utils/render.tsx",
        "src/Button.stories.tsx",
    ] {
        assert!(is_test_or_support_source(
            Language::TypeScript,
            Path::new(path)
        ));
        assert!(!is_test_source(Language::TypeScript, Path::new(path)));
    }
    assert!(is_test_or_support_source(
        Language::TypeScript,
        Path::new("src/a.test.ts")
    ));
    assert!(!is_test_or_support_source(
        Language::TypeScript,
        Path::new("src/lib/testing-utils.ts")
    ));
}
