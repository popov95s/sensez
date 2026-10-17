use std::path::Path;

pub(crate) fn language(path: &Path) -> tree_sitter::Language {
    match path.extension().and_then(|ext| ext.to_str()) {
        #[cfg(feature = "lang-typescript")]
        Some("ts" | "mts" | "cts") => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        #[cfg(feature = "lang-typescript")]
        Some("tsx") => tree_sitter_typescript::LANGUAGE_TSX.into(),
        _ => tree_sitter_javascript::LANGUAGE.into(),
    }
}
