//! The [`lang_profile!`] macro: one declaration per language profile.

/// Declare the parse/module/dead-code profile implementations for one language.
macro_rules! lang_profile {
    (
        $(#[$doc:meta])*
        $vis:vis struct $name:ident {
            info: $info:ident,
            language: $language:expr,
            extensions: $exts:expr,
            grammar: $grammar:expr,
            walk: $walk:path,
            root_for: $root_for:path,
            module_name: $module_name:path,
            is_package_index: $is_index:path,
            containing_package: $containing:path,
            resolve_target: $resolve_target:expr,
            submodule_candidate: $submodule:expr,
            decorators: $decorators:expr,
            classify_decorator: $classify:path,
            is_conventionally_private: $private:path,
            is_entry_file_stem: $entry_stem:path,
            dead_code_defaults: $dead_code_defaults:path,
            entry_modules: $entry_modules:expr,
            is_containment: $containment:expr,
        }
    ) => {
        static $info: $crate::profiles::LanguageInfo = $crate::profiles::LanguageInfo {
            language: $language,
            extensions: $exts,
        };

        $(#[$doc])*
        $vis struct $name;

        impl $crate::profiles::ParseProfile for $name {
            fn info(&self) -> &'static $crate::profiles::LanguageInfo {
                &$info
            }

            fn ts_language(&self) -> tree_sitter::Language {
                $grammar.into()
            }

            fn walk(
                &self,
                root: tree_sitter::Node,
                src: &[u8],
                file_id: u32,
                module_name: &str,
            ) -> $crate::spine::ir::Walked {
                $walk(root, src, file_id, module_name)
            }
        }

        impl $crate::profiles::ModuleProfile for $name {
            fn root_for(&self, file: &std::path::Path) -> std::path::PathBuf {
                $root_for(file)
            }

            fn module_name(&self, file: &std::path::Path, root: &std::path::Path) -> String {
                $module_name(file, root)
            }

            fn is_package_index(&self, file: &std::path::Path) -> bool {
                $is_index(file)
            }

            fn containing_package(&self, module_name: &str, is_index: bool) -> String {
                $containing(module_name, is_index)
            }

            fn resolve_target(
                &self,
                import: &$crate::spine::ir::ImportContext,
                importer_package: &str,
                importer_file: &std::path::Path,
                root: &std::path::Path,
            ) -> String {
                #[allow(clippy::redundant_closure_call)]
                ($resolve_target)(import, importer_package, importer_file, root)
            }

            fn submodule_candidate(&self, target: &str, symbol: &str) -> Option<String> {
                #[allow(clippy::redundant_closure_call)]
                ($submodule)(target, symbol)
            }

            fn is_containment(&self, importer: &str, target: &str) -> bool {
                #[allow(clippy::redundant_closure_call)]
                ($containment)(importer, target)
            }
        }

        impl $crate::profiles::DeadCodeProfile for $name {
            fn classify_decorator(
                &self,
                paths: Option<&Vec<String>>,
                user_entrypoints: &std::collections::HashSet<String>,
            ) -> $crate::profiles::DecoratorClass {
                $classify(paths, user_entrypoints)
            }

            fn is_conventionally_private(&self, symbol: &str) -> bool {
                $private(symbol)
            }

            fn is_entry_file_stem(&self, stem: &str) -> bool {
                $entry_stem(stem)
            }

            fn dead_code_defaults(&self) -> $crate::profiles::DeadCodeDefaults {
                $dead_code_defaults()
            }

            fn entry_modules(&self, project_root: &std::path::Path) -> Vec<String> {
                #[allow(clippy::redundant_closure_call)]
                ($entry_modules)(project_root)
            }
        }
    };
}

pub(crate) use lang_profile;
