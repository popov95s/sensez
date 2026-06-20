//! TypeScript language profiles. TS and TSX are distinct tree-sitter grammars
//! selected by extension, so each is its own zero-sized profile — both report
//! [`Language::TypeScript`] and reuse every JavaScript helper (the grammars
//! share node-kind names; TS-only kinds like `interface_declaration` simply map
//! to no structural token). TS decorators are a deferred enhancement.

use crate::profiles::javascript::{deadcode, resolve, roots, traversal};
use crate::profiles::profile_macro::lang_profile;
use crate::profiles::Language;

macro_rules! ts_profile {
    ($(#[$doc:meta])* $name:ident, $info:ident, $extensions:expr, $grammar:expr) => {
        lang_profile! {
            $(#[$doc])*
            pub struct $name {
                info: $info,
                language: Language::TypeScript,
                extensions: $extensions,
                grammar: $grammar,
                walk: traversal::walk,
                root_for: roots::root_for,
                module_name: roots::module_name,
                is_package_index: roots::is_package_index,
                containing_package: resolve::containing_package,
                resolve_target: |import, _pkg, file, root| resolve::resolve_target(import, file, root),
                submodule_candidate: |_target, _symbol| None,
                decorators: false,
                classify_decorator: deadcode::classify,
                is_conventionally_private: deadcode::is_conventionally_private,
                is_entry_file_stem: deadcode::is_entry_file_stem,
                dead_code_defaults: deadcode::typescript_defaults,
                entry_modules: |_root| Vec::new(),
                is_containment: |_importer, _target| false,
            }
        }
    };
}

ts_profile!(
    #[doc = "The TypeScript language profile (zero-sized)."]
    TsProfile,
    TS_INFO,
    &["ts"],
    tree_sitter_typescript::LANGUAGE_TYPESCRIPT
);
ts_profile!(
    #[doc = "The TSX language profile (zero-sized)."]
    TsxProfile,
    TSX_INFO,
    &["tsx"],
    tree_sitter_typescript::LANGUAGE_TSX
);

#[cfg(test)]
mod tests {
    use super::{TsProfile, TsxProfile};
    use crate::config::smells::{SmellConfig, Smells};
    use crate::noze::smells::detect_local;
    use crate::noze::SmellKind;
    use crate::profiles::Language;
    use crate::spine::parser::{parse_file, parse_source, ParsedFile, StructuralToken};
    use std::fs;
    use std::path::PathBuf;

    /// Build a `ParsedFile` for a TS source and return the smells `cfg` produces.
    fn smells_for(src: &[u8], cfg: &Smells) -> Vec<SmellKind> {
        let walked = parse_source(src, 0, "m", &TsProfile).unwrap();
        let file = ParsedFile {
            path: PathBuf::from("m.ts"),
            language: Language::TypeScript,
            lines: 0,
            walked,
        };
        detect_local(&file, cfg)
            .into_iter()
            .map(|f| f.kind)
            .collect()
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
