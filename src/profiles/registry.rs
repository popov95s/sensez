//! Profile registry + extension-based dispatch.
//!
//! `PROFILES` is assembled at compile time from whichever language features are
//! enabled. The default build enables every supported profile; smaller builds
//! can opt into individual language features.

use crate::profiles::wording::SmellWording;
use crate::profiles::{
    DeadCodeProfile, ImportResolver, Language, LanguageProfile, ModuleProfile, ParseProfile,
    PerformanceProfile, TypeVocabularyProfile,
};
use globset::GlobSet;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::sync::OnceLock;

/// All compiled-in language profiles (one zero-sized instance each).
static PROFILES: &[&dyn LanguageProfile] = &[
    #[cfg(feature = "lang-python")]
    &crate::profiles::python::PythonProfile,
    #[cfg(feature = "lang-javascript")]
    &crate::profiles::javascript::JsProfile,
    #[cfg(feature = "lang-typescript")]
    &crate::profiles::typescript::TsProfile,
    #[cfg(feature = "lang-typescript")]
    &crate::profiles::typescript::TsxProfile,
    #[cfg(feature = "lang-rust")]
    &crate::profiles::rust::RustProfile,
];

fn for_extension(ext: &str) -> Option<&'static dyn LanguageProfile> {
    PROFILES
        .iter()
        .copied()
        .find(|p| ParseProfile::info(*p).extensions.contains(&ext))
}

fn for_path(path: &Path) -> Option<&'static dyn LanguageProfile> {
    path.extension()
        .and_then(|e| e.to_str())
        .and_then(for_extension)
}

pub fn parse_for_path(path: &Path) -> Option<&'static dyn ParseProfile> {
    for_path(path).map(|profile| profile as &dyn ParseProfile)
}

pub fn should_parse_path(path: &Path) -> bool {
    parse_for_path(path).is_some_and(|profile| !profile.is_generated_or_data_source(path))
}

/// Whether an import target names machine-written source (by the importing language's path rules).
pub fn is_generated_import(language: Language, target: &str) -> bool {
    profile(language).is_generated(Path::new(target), b"")
}

pub fn is_declaration_only(path: &Path) -> bool {
    parse_for_path(path).is_some_and(|profile| profile.is_declaration_only(path))
}

fn profile(language: Language) -> &'static dyn LanguageProfile {
    match PROFILES
        .iter()
        .copied()
        .find(|p| ParseProfile::info(*p).language == language)
    {
        Some(profile) => profile,
        None => panic!("missing compiled-in profile for {language:?}"),
    }
}

pub fn module_profile(language: Language) -> &'static dyn ModuleProfile {
    profile(language) as &dyn ModuleProfile
}

/// Fresh import resolver state for every compiled-in language, for one graph build.
pub fn import_resolvers() -> BTreeMap<Language, Box<dyn ImportResolver>> {
    PROFILES
        .iter()
        .map(|p| ParseProfile::info(*p).language)
        .map(|language| (language, module_profile(language).import_resolver()))
        .collect()
}

pub fn dead_code_profile(language: Language) -> &'static dyn DeadCodeProfile {
    profile(language) as &dyn DeadCodeProfile
}

pub fn performance_profile(language: Language) -> &'static dyn PerformanceProfile {
    profile(language) as &dyn PerformanceProfile
}

pub fn clone_family(language: Language) -> Language {
    profile(language).clone_family()
}

pub fn type_vocabulary(language: Language) -> &'static dyn TypeVocabularyProfile {
    profile(language) as &dyn TypeVocabularyProfile
}

pub fn identifier_extra_chars(language: Language) -> &'static [char] {
    smell_wording(language).identifier_extra_chars
}

/// Extra identifier characters of `path`'s language; every compiled-in language's when it has none.
pub fn identifier_extra_chars_for_path(path: Option<&Path>) -> Vec<char> {
    match path.and_then(parse_for_path) {
        Some(profile) => profile.info().wording.identifier_extra_chars.to_vec(),
        None => PROFILES
            .iter()
            .flat_map(|profile| ParseProfile::info(*profile).wording.identifier_extra_chars)
            .copied()
            .collect(),
    }
}

pub fn smell_wording(language: Language) -> &'static SmellWording {
    profile(language).info().wording
}

/// Smells `language`'s profile switches off by default; none when it is not compiled in.
pub fn default_off_smells(language: Language) -> &'static [crate::report::SmellKind] {
    PROFILES
        .iter()
        .map(|profile| ParseProfile::info(*profile))
        .find(|info| info.language == language)
        .map_or(&[], |info| info.default_off_smells)
}

/// Migration-script globs of every compiled-in language.
pub fn migration_layouts() -> impl Iterator<Item = &'static str> {
    PROFILES
        .iter()
        .flat_map(|profile| ParseProfile::info(*profile).migration_layouts)
        .copied()
}

/// Match file conventions from the owning language profile, compiled once.
pub fn is_test_source(language: Language, path: &Path) -> bool {
    static TEST_SOURCES: OnceLock<HashMap<Language, GlobSet>> = OnceLock::new();
    TEST_SOURCES
        .get_or_init(|| {
            globs_by_language(|profile| DeadCodeProfile::dead_code_defaults(profile).test_sources)
        })
        .get(&language)
        .is_some_and(|patterns| patterns.is_match(path))
}

/// Test sources plus the profile's test-support layouts (mocks, fixtures, stories).
pub fn is_test_or_support_source(language: Language, path: &Path) -> bool {
    static TEST_SUPPORT: OnceLock<HashMap<Language, GlobSet>> = OnceLock::new();
    is_test_source(language, path)
        || TEST_SUPPORT
            .get_or_init(|| globs_by_language(|profile| ParseProfile::info(profile).test_support))
            .get(&language)
            .is_some_and(|patterns| patterns.is_match(path))
}

fn globs_by_language(
    patterns: impl Fn(&'static dyn LanguageProfile) -> &'static [&'static str],
) -> HashMap<Language, GlobSet> {
    PROFILES
        .iter()
        .map(|profile| {
            (
                ParseProfile::info(*profile).language,
                super::compile_profile_globs(patterns(*profile)),
            )
        })
        .collect()
}

#[cfg(all(
    test,
    any(
        feature = "lang-python",
        feature = "lang-javascript",
        feature = "lang-typescript"
    )
))]
mod tests {
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
}
