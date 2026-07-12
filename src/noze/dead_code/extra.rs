//! Opt-in dead-code passes: unused imports, methods, and properties.
//!
//! Unused imports are reliable intra-file checks. Class members use a
//! project-wide attribute evidence model so cross-file receiver access can keep
//! methods and properties live without pushing language-specific rules into the
//! shared graph or parser IR.

mod exports;
mod methods;
mod overrides;
mod properties;
mod properties_index;
mod usage;

use crate::profiles::registry;
use crate::report::{ActionLevel, Confidence, DeadCodeFinding};
use crate::spine::graph::CodebaseGraph;
use crate::spine::parser::SymbolKind;
use crate::spine::parser::{ImportPhase, ParsedFile};
use std::collections::HashMap;
use std::path::PathBuf;

pub(super) use methods::unused_methods;
pub use properties::unused_properties;

pub(super) struct MemberFiles<'a> {
    report: Vec<&'a ParsedFile>,
    usage: Vec<&'a ParsedFile>,
}

impl<'a> MemberFiles<'a> {
    pub(super) fn new(
        report: impl IntoIterator<Item = &'a ParsedFile>,
        usage: impl IntoIterator<Item = &'a ParsedFile>,
    ) -> Self {
        Self {
            report: report.into_iter().collect(),
            usage: usage.into_iter().collect(),
        }
    }

    #[cfg(test)]
    pub(super) fn same(files: impl IntoIterator<Item = &'a ParsedFile>) -> Self {
        let files: Vec<&ParsedFile> = files.into_iter().collect();
        Self {
            report: files.clone(),
            usage: files,
        }
    }

    pub(super) fn report(&self) -> &[&'a ParsedFile] {
        &self.report
    }

    pub(super) fn usage(&self) -> &[&'a ParsedFile] {
        &self.usage
    }
}

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
pub fn unused_imports<'a>(
    files: impl IntoIterator<Item = &'a ParsedFile>,
    modmap: &HashMap<PathBuf, String>,
) -> Vec<DeadCodeFinding> {
    let mut findings = Vec::new();
    for file in files {
        if registry::module_profile(file.language).is_package_index(&file.path) {
            continue;
        }
        let module = modmap.get(&file.path).cloned().unwrap_or_default();
        for import in file
            .walked
            .symbols
            .imports
            .iter()
            .filter(|i| !i.is_inline && i.phase == ImportPhase::Runtime)
        {
            for (index, binding) in import.bindings.iter().enumerate() {
                let phase = match import.binding_phases.get(index).copied() {
                    Some(phase) => phase,
                    None => import.phase,
                };
                if phase == ImportPhase::TypeOnly {
                    continue;
                }
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

fn is_referenced(counts: &HashMap<String, usize>, name: &str) -> bool {
    // Imports never contribute to name_counts, so any count means it is used.
    counts.get(name).copied().unwrap_or(0) > 0
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

        let member_files = MemberFiles::same(files.iter());
        let meths: Vec<_> = unused_methods(&member_files, &modmap, |_language, _name| false)
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

    #[test]
    fn ignores_type_checking_only_imports() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_path_buf();
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("m.py"),
            "from typing import TYPE_CHECKING\n\nif TYPE_CHECKING:\n    from app.models import User\n\nimport os\n",
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
            !imps.contains(&"User".to_string()),
            "type-checker-only imports are intentional"
        );
        assert!(imps.contains(&"os".to_string()), "ordinary unused import");
    }

    #[cfg(feature = "lang-typescript")]
    #[test]
    fn ignores_type_only_bindings_but_reports_runtime_bindings() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_path_buf();
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("consumer.ts"),
            "import { type MassiveUserClass, connect as runtimeConnect } from './models';\n",
        )
        .unwrap();
        fs::write(
            dir.join("models.ts"),
            "export class MassiveUserClass {}\nexport function connect(): void {}\n",
        )
        .unwrap();
        let files = vec![
            parse_file(&dir.join("consumer.ts"), 0).unwrap(),
            parse_file(&dir.join("models.ts"), 1).unwrap(),
        ];
        let cg = crate::spine::graph::build(&files, &[]);
        let modmap = module_map(&cg);

        let imps: Vec<_> = unused_imports(&files, &modmap)
            .iter()
            .map(|f| f.symbol.clone())
            .collect();
        assert!(!imps.contains(&"MassiveUserClass".to_string()));
        assert!(imps.contains(&"runtimeConnect".to_string()));
    }
}
