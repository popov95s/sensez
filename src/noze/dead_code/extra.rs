//! Opt-in dead-code passes: unused imports and unused methods.
//!
//! Both are intra-file analyses keyed off the per-file identifier counts:
//! a binding/method whose name never appears beyond its definition site is a
//! candidate. Unused imports are reliably detectable (High); unused methods are
//! Low confidence because cross-file attribute access (`obj.method()`) cannot
//! be seen statically.

use crate::noze::{ActionLevel, Confidence, DeadCodeFinding, SymbolKind};
use crate::profiles::{registry, Language};
use crate::spine::graph::CodebaseGraph;
use crate::spine::parser::ParsedFile;
use std::collections::HashMap;
use std::path::PathBuf;

/// Map each internal file path to its resolved dotted module name.
pub fn module_map(cg: &CodebaseGraph) -> HashMap<PathBuf, String> {
    cg.graph
        .node_indices()
        .map(|i| &cg.graph[i])
        .filter(|n| !n.is_external)
        .map(|n| (n.file_path.clone(), n.module_name.clone()))
        .collect()
}

/// Imports whose bound name is never referenced in the file. `__init__.py` is
/// skipped (its imports are typically intentional re-exports).
pub fn unused_imports(
    files: &[ParsedFile],
    modmap: &HashMap<PathBuf, String>,
) -> Vec<DeadCodeFinding> {
    let mut findings = Vec::new();
    for file in files {
        if registry::module_profile(file.language).is_package_index(&file.path) {
            continue;
        }
        let module = modmap.get(&file.path).cloned().unwrap_or_default();
        for import in file.walked.symbols.imports.iter().filter(|i| !i.is_inline) {
            for binding in &import.bindings {
                if binding == "*" || is_referenced(&file.walked.usage.name_counts, binding) {
                    continue;
                }
                if file
                    .walked
                    .symbols
                    .dunder_all
                    .as_ref()
                    .is_some_and(|a| a.iter().any(|s| s == binding))
                {
                    continue;
                }
                findings.push(DeadCodeFinding {
                    action: ActionLevel::Advisory,
                    module: module.clone(),
                    symbol: binding.clone(),
                    kind: SymbolKind::Import,
                    confidence: Confidence::High,
                    file: file.path.clone(),
                    line: import.line,
                    reason: String::new(),
                });
            }
        }
    }
    findings
}

/// Methods never referenced within their own module. Dunders and configured
/// entrypoint names are excluded; results are Low confidence.
pub fn unused_methods(
    files: &[ParsedFile],
    modmap: &HashMap<PathBuf, String>,
    is_entrypoint_name: impl Fn(Language, &str) -> bool,
) -> Vec<DeadCodeFinding> {
    let mut findings = Vec::new();
    for file in files {
        let module = modmap.get(&file.path).cloned().unwrap_or_default();
        for (name, line) in &file.walked.symbols.methods {
            if is_dunder(name) || is_entrypoint_name(file.language, name) {
                continue;
            }
            if file
                .walked
                .usage
                .name_counts
                .get(name)
                .copied()
                .unwrap_or(0)
                > 1
            {
                continue; // called somewhere in this module
            }
            findings.push(DeadCodeFinding {
                action: ActionLevel::Advisory,
                module: module.clone(),
                symbol: name.clone(),
                kind: SymbolKind::Method,
                confidence: Confidence::Low,
                file: file.path.clone(),
                line: *line,
                reason: String::new(),
            });
        }
    }
    findings
}

fn is_referenced(counts: &HashMap<String, usize>, name: &str) -> bool {
    // Imports never contribute to name_counts, so any count means it is used.
    counts.get(name).copied().unwrap_or(0) > 0
}

fn is_dunder(name: &str) -> bool {
    name.starts_with("__") && name.ends_with("__")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spine::parser::parse_file;
    use std::fs;

    #[test]
    fn detects_unused_import_and_method() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_path_buf();
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("m.py"),
            "import os\nimport sys\n\nsys.exit(0)\n\nclass C:\n    def used(self):\n        return self.unused()\n    def unused(self):\n        return 1\n    def orphan(self):\n        return 2\n",
        )
        .unwrap();
        let files = vec![parse_file(&dir.join("m.py"), 0).unwrap()];
        let cg = crate::spine::graph::build(&files, &[]);
        let modmap = module_map(&cg);

        let imps: Vec<_> = unused_imports(&files, &modmap)
            .iter()
            .map(|f| f.symbol.clone())
            .collect();
        assert!(
            imps.contains(&"os".to_string()),
            "os is imported but unused"
        );
        assert!(!imps.contains(&"sys".to_string()), "sys is used");

        let meths: Vec<_> = unused_methods(&files, &modmap, |_language, _name| false)
            .iter()
            .map(|f| f.symbol.clone())
            .collect();
        assert!(
            meths.contains(&"orphan".to_string()),
            "orphan() is never called"
        );
        assert!(
            !meths.contains(&"unused".to_string()),
            "unused() is called via self"
        );
    }
}
