use std::path::Path;

/// TypeScript declaration files (`.d.ts`, `.d.mts`, `.d.cts`) carry types only.
pub(crate) fn is_declaration_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            [".d.ts", ".d.mts", ".d.cts"]
                .iter()
                .any(|suffix| name.ends_with(suffix))
        })
}
