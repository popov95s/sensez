//! Module keys for a test file's imports and mock specifiers, resolved alike.

use crate::profiles::{registry, ImportResolver, ModuleProfile};
use crate::spine::ir::{ImportContext, ImportPhase};
use crate::spine::parser::ParsedFile;
use std::path::{Path, PathBuf};

/// One imported name the test calls, and the module member the call reaches.
pub(super) struct ReachedMember {
    pub binding: String,
    pub module: String,
    pub member: Option<String>,
}

pub(super) struct ModuleKeys<'a> {
    path: &'a Path,
    profile: &'static dyn ModuleProfile,
    resolver: Box<dyn ImportResolver>,
    root: PathBuf,
    package: String,
}

impl<'a> ModuleKeys<'a> {
    pub fn for_file(file: &'a ParsedFile) -> Self {
        let profile = registry::module_profile(file.language);
        let root = profile.root_for(&file.path);
        let name = profile.module_name(&file.path, &root);
        Self {
            path: &file.path,
            profile,
            resolver: profile.import_resolver(),
            package: profile.containing_package(&name, profile.is_package_index(&file.path)),
            root,
        }
    }

    pub fn import(&mut self, import: &ImportContext) -> String {
        self.resolver
            .resolve_target(import, &self.package, self.path, &self.root)
    }

    pub fn specifier(&mut self, specifier: &str) -> String {
        self.import(&ImportContext {
            source_module: String::new(),
            target_module: specifier.to_string(),
            imported_symbols: Vec::new(),
            bindings: Vec::new(),
            binding_phases: Vec::new(),
            line: 0,
            column: 0,
            phase: ImportPhase::Runtime,
            is_inline: false,
            is_module_decl: false,
            is_reexport: false,
            enclosing_scope: None,
        })
    }

    /// Members reached by `call` paths (`name`, `name.member`) through `import`.
    pub fn reached(&mut self, import: &ImportContext, calls: &[&str]) -> Vec<ReachedMember> {
        let hits: Vec<(usize, Option<&str>)> = calls
            .iter()
            .filter_map(|call| {
                let mut segments = call.split('.');
                let root = segments.next()?;
                let index = import.bindings.iter().position(|binding| binding == root)?;
                Some((index, segments.next()))
            })
            .collect();
        if hits.is_empty() {
            return Vec::new();
        }
        let module = self.import(import);
        hits.into_iter()
            .flat_map(|(index, member)| self.members(import, index, &module, member))
            .collect()
    }

    fn members(
        &self,
        import: &ImportContext,
        index: usize,
        module: &str,
        member: Option<&str>,
    ) -> Vec<ReachedMember> {
        let binding = &import.bindings[index];
        let symbol = import
            .imported_symbols
            .get(index)
            .filter(|symbol| symbol.as_str() != "*");
        let submodule = symbol
            .and_then(|symbol| self.profile.submodule_candidate(module, symbol))
            .zip(member)
            .map(|(submodule, member)| ReachedMember {
                binding: binding.clone(),
                module: submodule,
                member: Some(member.to_string()),
            });
        let direct = ReachedMember {
            binding: binding.clone(),
            module: module.to_string(),
            member: symbol.cloned().or(member.map(str::to_string)),
        };
        submodule.into_iter().chain([direct]).collect()
    }
}
