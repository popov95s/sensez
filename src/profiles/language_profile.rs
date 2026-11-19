use super::dead_code_profile::DeadCodeProfile;
use super::dependencies::DependencyProfile;
use super::module_profile::ModuleProfile;
use super::parse_profile::ParseProfile;
use super::performance_profile::PerformanceProfile;
use super::type_vocabulary_profile::TypeVocabularyProfile;

pub trait LanguageProfile:
    ParseProfile
    + ModuleProfile
    + DeadCodeProfile
    + PerformanceProfile
    + TypeVocabularyProfile
    + DependencyProfile
{
}

impl<T> LanguageProfile for T where
    T: ParseProfile
        + ModuleProfile
        + DeadCodeProfile
        + PerformanceProfile
        + TypeVocabularyProfile
        + DependencyProfile
{
}
