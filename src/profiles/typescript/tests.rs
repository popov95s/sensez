use super::{TsProfile, TsxProfile};
use crate::config::smells::{SmellConfig, Smells};
use crate::noze::smells::detect_local;
use crate::report::{SmellFinding, SmellKind};
use crate::spine::ir::Language;
use crate::spine::parser::{parse_file, parse_source, ImportPhase, ParsedFile, StructuralToken};
use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn graph_resolves_tsconfig_paths_and_keeps_workspace_module_ids_unique() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    fs::write(dir.join("package.json"), "{\"name\":\"workspace\"}\n").unwrap();
    fs::write(
        dir.join("tsconfig.json"),
        r#"{ "compilerOptions": { "paths": { "@/*": ["src/*"] } } }"#,
    )
    .unwrap();
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::create_dir_all(dir.join("packages/alpha/src")).unwrap();
    fs::create_dir_all(dir.join("packages/beta/src")).unwrap();
    fs::write(dir.join("src/data.ts"), "export const live = 1;\n").unwrap();
    fs::write(
        dir.join("src/consumer.ts"),
        "import { live } from '@/data';\nconsole.log(live);\n",
    )
    .unwrap();
    fs::write(
        dir.join("packages/alpha/src/index.ts"),
        "export const alpha = 1;\n",
    )
    .unwrap();
    fs::write(
        dir.join("packages/beta/src/index.ts"),
        "export const beta = 1;\n",
    )
    .unwrap();

    let files: Vec<_> = [
        "src/data.ts",
        "src/consumer.ts",
        "packages/alpha/src/index.ts",
        "packages/beta/src/index.ts",
    ]
    .iter()
    .enumerate()
    .map(|(index, file)| parse_file(&dir.join(file), index as u32).unwrap())
    .collect();
    let graph = crate::spine::graph::build(&files, &[]);

    let consumer = graph.name_to_index["src/consumer"];
    let data = graph.name_to_index["src/data"];
    assert!(graph.graph.find_edge(consumer, data).is_some());
    assert!(graph.name_to_index.contains_key("packages/alpha/src"));
    assert!(graph.name_to_index.contains_key("packages/beta/src"));
}

/// Build a `ParsedFile` for a TS source and return the smells `cfg` produces.
fn findings_for(src: &[u8], cfg: &Smells) -> Vec<SmellFinding> {
    let walked = parse_source(src, 0, "m", &TsProfile).unwrap();
    let file = ParsedFile {
        path: PathBuf::from("m.ts"),
        language: Language::TypeScript,
        lines: 0,
        fingerprint: crate::spine::cache::SourceFingerprint::new(
            Path::new("m.ts"),
            Language::TypeScript,
            src,
        ),
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
    let src =
        b"export function f(cfg: Record<string, any>): Record<string, any> {\n  return cfg;\n}\n";
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

#[test]
fn ts_loose_typing_high_reports_arrays_and_aliases() {
    let src = b"type UserId = string;\nexport function f(ids: string[]): void {}\n";
    let cfg = Smells {
        loose_typing_strictness: crate::config::smells::Strictness::High,
        ..Smells::default()
    };
    let findings = findings_for(src, &cfg);
    assert!(
        findings
            .iter()
            .any(|f| f.kind == SmellKind::LooseTyping && f.message.contains("ids")),
        "{findings:?}"
    );
    assert!(
        findings
            .iter()
            .any(|f| f.kind == SmellKind::LooseTyping && f.message.contains("type alias UserId")),
        "{findings:?}"
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

#[test]
fn fused_collector_ts_return_annotation_not_in_body() {
    let src = b"function add(a: number, b: number): number {\n  return a + b;\n}\n";
    let functions = parse_source(src, 0, "m", &TsProfile)
        .unwrap()
        .units
        .functions;
    assert_eq!(functions.len(), 1);
    let f = &functions[0];
    assert_eq!(f.name, "add");
    assert_eq!(f.return_count, 1);
    assert_eq!(f.cognitive, 0);
    assert_eq!(f.branch_count, 0);
    assert_eq!(f.max_nesting, 0);
    assert_eq!(f.magic_numbers, 0);
}
