use crate::profiles::javascript::{test_support, wording};
use crate::profiles::runner_config::RunnerKind;
use crate::profiles::LanguageInfo;
use crate::spine::ir::Language;

pub(super) static TS_INFO: LanguageInfo = LanguageInfo {
    language: Language::TypeScript,
    extensions: &["ts", "mts", "cts"],
    test_support: test_support::TEST_SUPPORT,
    migration_layouts: &[],
    test_runners: &[RunnerKind::Vitest, RunnerKind::Jest],
    default_off_smells: &[],
    #[cfg(feature = "lsp")]
    project_files: &["package.json", "tsconfig*.json"],
    import_words: &["import", "require", "from"],
    wording: &wording::WORDING,
};

pub(super) static TSX_INFO: LanguageInfo = LanguageInfo {
    language: Language::TypeScript,
    extensions: &["tsx"],
    test_support: test_support::TEST_SUPPORT,
    migration_layouts: &[],
    test_runners: &[RunnerKind::Vitest, RunnerKind::Jest],
    default_off_smells: &[],
    #[cfg(feature = "lsp")]
    project_files: &["package.json", "tsconfig*.json"],
    import_words: &["import", "require", "from"],
    wording: &wording::WORDING,
};
