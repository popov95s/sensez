//! Profile registry + extension-based dispatch.
//!
//! `PROFILES` is assembled at compile time from whichever language features are
//! enabled. The default build enables every supported profile; smaller builds
//! can opt into individual language features.

use crate::profiles::{DeadCodeProfile, Language, LanguageProfile, ModuleProfile, ParseProfile};
use std::path::Path;

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

pub fn dead_code_profile(language: Language) -> &'static dyn DeadCodeProfile {
    profile(language) as &dyn DeadCodeProfile
}
