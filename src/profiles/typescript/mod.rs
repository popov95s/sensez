//! TypeScript language profiles. TS and TSX are distinct tree-sitter grammars
//! selected by extension, so each is its own zero-sized profile — both report
//! [`Language::TypeScript`] and reuse every JavaScript helper (the grammars
//! share node-kind names; TS-only kinds like `interface_declaration` simply map
//! to no structural token). TS decorators are a deferred enhancement.

use crate::profiles::javascript::{deadcode, performance, resolve, roots, traversal};
use crate::profiles::{
    DeadCodeProfile, Language, LanguageInfo, ModuleProfile, ParseProfile, PerformanceProfile,
};
use crate::spine::ir::{ImportContext, Walked};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

static TS_INFO: LanguageInfo = LanguageInfo {
    language: Language::TypeScript,
    extensions: &["ts"],
};

static TSX_INFO: LanguageInfo = LanguageInfo {
    language: Language::TypeScript,
    extensions: &["tsx"],
};

/// The TypeScript language profile (zero-sized).
pub struct TsProfile;

impl ParseProfile for TsProfile {
    fn info(&self) -> &'static LanguageInfo {
        &TS_INFO
    }

    fn ts_language(&self) -> tree_sitter::Language {
        tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
    }

    fn walk(
        &self,
        root: tree_sitter::Node,
        src: &[u8],
        file_id: u32,
        module_name: &str,
    ) -> Walked {
        traversal::walk(root, src, file_id, module_name)
    }
}

/// The TSX language profile (zero-sized).
pub struct TsxProfile;

impl ParseProfile for TsxProfile {
    fn info(&self) -> &'static LanguageInfo {
        &TSX_INFO
    }

    fn ts_language(&self) -> tree_sitter::Language {
        tree_sitter_typescript::LANGUAGE_TSX.into()
    }

    fn walk(
        &self,
        root: tree_sitter::Node,
        src: &[u8],
        file_id: u32,
        module_name: &str,
    ) -> Walked {
        traversal::walk(root, src, file_id, module_name)
    }
}

// TS and TSX share everything except the language info and the underlying
// tree-sitter grammar, so the rest of the trait impls are identical and
// delegated to a single generic helper. The macro equivalent used to
// duplicate this; the duplication is gone.
macro_rules! impl_ts_traits {
    ($name:ident) => {
        impl ModuleProfile for $name {
            fn root_for(&self, file: &Path) -> PathBuf {
                roots::root_for(file)
            }

            fn module_name(&self, file: &Path, root: &Path) -> String {
                roots::module_name(file, root)
            }

            fn is_package_index(&self, file: &Path) -> bool {
                roots::is_package_index(file)
            }

            fn containing_package(&self, module_name: &str, is_index: bool) -> String {
                resolve::containing_package(module_name, is_index)
            }

            fn resolve_target(
                &self,
                import: &ImportContext,
                _importer_package: &str,
                file: &Path,
                root: &Path,
            ) -> String {
                resolve::resolve_target(import, file, root)
            }

            fn submodule_candidate(&self, _target: &str, _symbol: &str) -> Option<String> {
                None
            }

            fn is_containment(&self, _importer: &str, _target: &str) -> bool {
                false
            }
        }

        impl DeadCodeProfile for $name {
            fn classify_decorator(
                &self,
                paths: Option<&Vec<String>>,
                user_entrypoints: &HashSet<String>,
            ) -> crate::profiles::DecoratorClass {
                deadcode::classify(paths, user_entrypoints)
            }

            fn is_conventionally_private(&self, symbol: &str) -> bool {
                deadcode::is_conventionally_private(symbol)
            }

            fn is_entry_file_stem(&self, stem: &str) -> bool {
                deadcode::is_entry_file_stem(stem)
            }

            fn dead_code_defaults(&self) -> crate::profiles::DeadCodeDefaults {
                deadcode::typescript_defaults()
            }

            fn entry_modules(&self, _project_root: &Path) -> Vec<String> {
                Vec::new()
            }
        }

        impl PerformanceProfile for $name {
            fn is_expensive_loop_call(&self, method: &str) -> bool {
                performance::EXPENSIVE_LOOP_METHODS.contains(&method)
            }

            fn is_external_get_receiver(&self, base: &str) -> bool {
                performance::EXTERNAL_GET_RECEIVERS.contains(&base)
            }
        }
    };
}

impl_ts_traits!(TsProfile);
impl_ts_traits!(TsxProfile);

#[cfg(test)]
mod tests {
    use super::{TsProfile, TsxProfile};
    use crate::config::smells::{SmellConfig, Smells};
    use crate::noze::smells::detect_local;
    use crate::report::{SmellFinding, SmellKind};
    use crate::spine::ir::Language;
    use crate::spine::parser::{
        parse_file, parse_source, ImportPhase, ParsedFile, StructuralToken,
    };
    use std::fs;
    use std::path::PathBuf;

    /// Build a `ParsedFile` for a TS source and return the smells `cfg` produces.
    fn findings_for(src: &[u8], cfg: &Smells) -> Vec<SmellFinding> {
        let walked = parse_source(src, 0, "m", &TsProfile).unwrap();
        let file = ParsedFile {
            path: PathBuf::from("m.ts"),
            language: Language::TypeScript,
            lines: 0,
            walked,
        };
        detect_local(&file, cfg)
    }

    fn smells_for(src: &[u8], cfg: &Smells) -> Vec<SmellKind> {
        findings_for(src, cfg).into_iter().map(|f| f.kind).collect()
    }

    #[test]
    fn import_type_is_type_only_phase() {
        let imports = parse_source(
            b"import type { MassiveUserClass } from './heavy_database_models';\nimport { live } from './runtime';\n",
            0,
            "m",
            &TsProfile,
        )
        .unwrap()
        .symbols
        .imports;

        let type_only = imports
            .iter()
            .find(|i| i.target_module == "./heavy_database_models")
            .unwrap();
        assert_eq!(type_only.phase, ImportPhase::TypeOnly);
        assert_eq!(type_only.imported_symbols, vec!["MassiveUserClass"]);

        let runtime = imports
            .iter()
            .find(|i| i.target_module == "./runtime")
            .unwrap();
        assert_eq!(runtime.phase, ImportPhase::Runtime);
    }

    #[test]
    fn mixed_import_specifiers_track_per_binding_phase() {
        let imports = parse_source(
            b"import { type MassiveUserClass, connect as runtimeConnect } from './heavy_database_models';\n",
            0,
            "m",
            &TsProfile,
        )
        .unwrap()
        .symbols
        .imports;

        let import = imports
            .iter()
            .find(|i| i.target_module == "./heavy_database_models")
            .unwrap();
        assert_eq!(import.phase, ImportPhase::Runtime);
        assert_eq!(import.imported_symbols, vec!["MassiveUserClass", "connect"]);
        assert_eq!(import.bindings, vec!["MassiveUserClass", "runtimeConnect"]);
        assert_eq!(
            import.binding_phases,
            vec![ImportPhase::TypeOnly, ImportPhase::Runtime]
        );
    }

    #[test]
    fn export_type_is_type_only_phase() {
        let imports = parse_source(
            b"export type { Shape } from './shape';\n",
            0,
            "m",
            &TsProfile,
        )
        .unwrap()
        .symbols
        .imports;
        assert_eq!(imports[0].phase, ImportPhase::TypeOnly);
        assert_eq!(imports[0].target_module, "./shape");
    }

    /// The type-discipline + mutation smells fire for TypeScript via the new
    /// unit/type-hint extraction; the ESLint-owned smells are suppressed by the
    /// built-in TS default but can be re-enabled per language.
    #[test]
    fn ts_smells_fire_and_defaults_gate_eslint_owned() {
        let src = b"export function handle(cfg: Record<string, any>, a: boolean, b: boolean, c: boolean): [string, number, boolean] {\n  if (a) { if (b) { if (c) { if (cfg) { if (a) { return [\"x\", 7, true]; } } } } }\n  return [\"y\", 8, false];\n}\nexport function pump(items: any[]): void { items.push(1); }\nexport function coerce(name?: string): string { return name || \"\"; }\nexport function fallback(name?: string): string { return name ? name : \"?\"; }\n";

        let defaults = SmellConfig::default();
        let kinds = smells_for(src, defaults.for_language(Language::TypeScript));
        assert!(kinds.contains(&SmellKind::LooseTyping), "{kinds:?}");
        assert!(kinds.contains(&SmellKind::BooleanBlindness), "{kinds:?}");
        assert!(kinds.contains(&SmellKind::TuplePacking), "{kinds:?}");
        assert!(kinds.contains(&SmellKind::MutatedParameter), "{kinds:?}");
        assert!(kinds.contains(&SmellKind::MagicStringDefault), "{kinds:?}");
        assert!(kinds.contains(&SmellKind::UnnecessaryNestedIf), "{kinds:?}");
        // ESLint/SonarJS own these — off by the TS default.
        assert!(!kinds.contains(&SmellKind::DeepNesting), "{kinds:?}");
        assert!(!kinds.contains(&SmellKind::MagicNumbers), "{kinds:?}");

        // Per-language override re-enables them.
        let enabled = Smells {
            disabled: Vec::new(),
            magic_numbers: true,
            ..Smells::default()
        };
        let kinds = smells_for(src, &enabled);
        assert!(kinds.contains(&SmellKind::DeepNesting), "{kinds:?}");
        assert!(kinds.contains(&SmellKind::MagicNumbers), "{kinds:?}");
    }

    #[test]
    fn ts_loose_typing_uses_language_specific_suggestion() {
        let src = b"export function f(cfg: Record<string, any>): Record<string, any> {\n  return cfg;\n}\n";
        let findings = findings_for(src, &Smells::default());
        let loose = findings
            .into_iter()
            .find(|f| f.kind == SmellKind::LooseTyping)
            .expect("must flag loose typing");
        assert!(
            loose.message.contains("typed object") || loose.message.contains("interface"),
            "{}",
            loose.message
        );
        assert!(
            !loose.message.contains("dataclass"),
            "JS/TS wording should not mention dataclass: {}",
            loose.message
        );
    }

    /// TS type annotations/interfaces don't break structural tokenization, and
    /// the control-flow shape is still captured.
    #[test]
    fn typed_function_yields_structural_tokens() {
        let src = b"interface U { id: number }\nexport function f(xs: number[]): number {\n  let n: number = 0;\n  for (const x of xs) { if (x > 0) { n = n + x; } }\n  return n;\n}\n";
        let toks = parse_source(src, 0, "m", &TsProfile).unwrap().syntax.tokens;
        assert!(toks.contains(&StructuralToken::FunctionDef));
        assert!(toks.contains(&StructuralToken::ForStatement));
        assert!(toks.contains(&StructuralToken::Return));
    }

    /// TSX grammar parses and tokenizes (JSX collapses to no structural token).
    #[test]
    fn tsx_parses() {
        let src = b"export function View() { return foo(); }\n";
        let toks = parse_source(src, 0, "m", &TsxProfile)
            .unwrap()
            .syntax
            .tokens;
        assert!(toks.contains(&StructuralToken::FunctionDef));
    }

    /// `.ts` relative imports resolve to sibling module keys (internal edge).
    #[test]
    fn ts_relative_import_resolves_internal() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_path_buf();
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::write(dir.join("package.json"), "{\"name\":\"x\"}\n").unwrap();
        fs::write(
            dir.join("src/models.ts"),
            "export function makeUser(): number { return 1; }\n",
        )
        .unwrap();
        fs::write(
            dir.join("src/service.ts"),
            "import { makeUser } from './models';\nexport function build() { return makeUser(); }\n",
        )
        .unwrap();

        let files: Vec<_> = ["src/models.ts", "src/service.ts"]
            .iter()
            .enumerate()
            .map(|(i, n)| parse_file(&dir.join(n), i as u32).unwrap())
            .collect();
        let cg = crate::spine::graph::build(&files, &[]);
        let service = cg.name_to_index["src/service"];
        let models = cg.name_to_index["src/models"];
        assert!(
            cg.graph.find_edge(service, models).is_some(),
            "service.ts -> models.ts must resolve internally"
        );
    }

    #[test]
    fn python_class_base_defaults_do_not_leak_to_typescript() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_path_buf();
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("plugin.ts"),
            "export class Base {}\nexport class AdminConfig extends AppConfig {}\n",
        )
        .unwrap();

        let files = vec![parse_file(&dir.join("plugin.ts"), 0).unwrap()];
        let cg = crate::spine::graph::build(&files, &[]);
        let dead: Vec<_> = crate::noze::dead_code::detect(
            &cg,
            &files,
            &crate::config::model::Config::default().dead_code,
        )
        .iter()
        .map(|f| f.symbol.clone())
        .collect();

        assert!(
            dead.contains(&"AdminConfig".to_string()),
            "TypeScript must not inherit Python's AppConfig entrypoint base"
        );
    }
}
