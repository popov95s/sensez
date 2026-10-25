use crate::config::model::{Config, DeadCode};
use crate::noze::dead_code::detect;
use crate::report::{Confidence, DeadCodeFinding};
use crate::spine::parser::{parse_file, SymbolKind};
use std::fs;

pub(super) fn scan(sources: &[(&str, &str)], config: &DeadCode) -> Vec<DeadCodeFinding> {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("package.json"), "{}").unwrap();
    let paths: Vec<_> = sources
        .iter()
        .map(|(name, source)| {
            let path = tmp.path().join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, source).unwrap();
            path
        })
        .collect();
    let files: Vec<_> = paths
        .iter()
        .filter(|path| path.extension().is_some_and(|ext| ext != "json"))
        .enumerate()
        .map(|(index, path)| parse_file(path, index as u32).unwrap())
        .collect();
    let cg = crate::spine::graph::build(&files, &[]);
    detect(&cg, &files, config)
}

/// Confidence of the finding for `symbol`; `None` when absent or Low.
pub(super) fn kept_confidence(findings: &[DeadCodeFinding], symbol: &str) -> Option<Confidence> {
    findings
        .iter()
        .find(|finding| finding.symbol == symbol && finding.confidence != Confidence::Low)
        .map(|finding| finding.confidence)
}

#[test]
fn consumed_reexports_are_not_unused_imports() {
    let mut config = Config::default().dead_code;
    config.unused_imports = true;
    let findings = scan(
        &[
            (
                "src/helper.ts",
                "export function helper() {}\nexport function orphan() {}\n",
            ),
            (
                "src/public.ts",
                "export { helper, orphan } from \"./helper\";\n",
            ),
            ("src/m.ts", "export const a = 1;\nexport const b = 2;\n"),
            (
                "src/consumer.ts",
                "import { helper } from \"./public\";\nimport { a, b } from \"./m\";\nhelper(a);\n",
            ),
        ],
        &config,
    );
    let imports: Vec<_> = findings
        .iter()
        .filter(|finding| finding.kind == SymbolKind::Import)
        .map(|finding| finding.symbol.as_str())
        .collect();

    assert_eq!(imports, vec!["b"]);
}

fn method_findings() -> Vec<DeadCodeFinding> {
    let mut config = Config::default().dead_code;
    config.unused_methods = true;
    scan(
        &[
            (
                "src/boundary.tsx",
                "export class Boundary extends Component {\n  static getDerivedStateFromError() { return {}; }\n  componentDidCatch() {}\n  render() { return null; }\n  neverCalled() { return 1; }\n}\n",
            ),
            (
                "src/reporter.ts",
                "export class Reporter {\n  onTestEnd() {}\n  onBegin() {}\n  helperUnused() {}\n}\nexport class R implements X {\n  onEnd() {}\n  finish() {}\n}\n",
            ),
            (
                "src/plain.ts",
                "class C {\n  used() { return this.x(); }\n  x() { return 1; }\n  orphan() { return 2; }\n}\nexport function run() { const c: C = new C(); return c.used(); }\n",
            ),
            (
                "src/use.ts",
                "import { Boundary } from \"./boundary\";\nimport { Reporter, R } from \"./reporter\";\nimport { run } from \"./plain\";\nconst b: Boundary = new Boundary();\nb.render();\nconst r: Reporter = new Reporter();\nr.onBegin();\nconst e: R = new R();\ne.onEnd();\nrun();\n",
            ),
        ],
        &config,
    )
}

#[test]
fn react_lifecycle_methods_are_not_reported() {
    let findings = method_findings();
    for symbol in ["getDerivedStateFromError", "componentDidCatch"] {
        assert_eq!(kept_confidence(&findings, symbol), None, "{symbol}");
    }
}

#[test]
fn methods_of_classes_outside_the_scan_are_not_reported() {
    let findings = method_findings();
    assert_eq!(kept_confidence(&findings, "neverCalled"), None);
}

#[test]
fn runner_callback_names_are_not_reported() {
    let findings = method_findings();
    assert_eq!(kept_confidence(&findings, "onTestEnd"), None);
}

#[test]
fn methods_of_implemented_interfaces_are_not_reported() {
    let findings = method_findings();
    assert_eq!(kept_confidence(&findings, "finish"), None);
}

#[test]
fn unused_plain_methods_are_reported() {
    let findings = method_findings();
    assert_eq!(kept_confidence(&findings, "orphan"), Some(Confidence::High));
    assert_eq!(
        kept_confidence(&findings, "helperUnused"),
        Some(Confidence::High)
    );
}

#[test]
fn callback_names_do_not_silence_top_level_functions() {
    let findings = scan(
        &[
            (
                "src/view.ts",
                "export function render() {}\nexport function onError() {}\nexport function used() {}\n",
            ),
            ("src/app.ts", "import { used } from \"./view\";\nused();\n"),
        ],
        &Config::default().dead_code,
    );

    assert_eq!(kept_confidence(&findings, "render"), Some(Confidence::High));
    assert_eq!(
        kept_confidence(&findings, "onError"),
        Some(Confidence::High)
    );
}

#[test]
fn entrypoint_names_accept_prefix_patterns_for_methods() {
    let sources = [
        (
            "src/hooks.ts",
            "export class Hooks {\n  onCustom() {}\n  stale() {}\n}\n",
        ),
        (
            "src/use.ts",
            "import { Hooks } from \"./hooks\";\nconst h: Hooks = new Hooks();\nh.stale;\n",
        ),
    ];
    let mut config = Config::default().dead_code;
    config.unused_methods = true;
    assert!(kept_confidence(&scan(&sources, &config), "onCustom").is_some());

    config.entrypoint_names = vec!["on*".to_string()];
    assert_eq!(kept_confidence(&scan(&sources, &config), "onCustom"), None);
}

#[test]
fn namespace_imports_credit_member_access() {
    let findings = scan(
        &[
            (
                "src/lib.ts",
                "export function used() {}\nexport function other() {}\n",
            ),
            ("src/app.ts", "import * as ns from \"./lib\";\nns.used();\n"),
        ],
        &Config::default().dead_code,
    );

    assert_eq!(kept_confidence(&findings, "used"), None);
    assert!(kept_confidence(&findings, "other").is_some());
}

#[test]
fn namespace_value_escape_keeps_module_alive() {
    let findings = scan(
        &[
            (
                "src/lib.ts",
                "export function used() {}\nexport function other() {}\n",
            ),
            (
                "src/app.ts",
                "import * as ns from \"./lib\";\nns.used();\nconsume(ns);\n",
            ),
        ],
        &Config::default().dead_code,
    );

    assert!(findings.is_empty(), "{findings:?}");
}

#[test]
fn namespace_reexports_stay_star() {
    let findings = scan(
        &[
            ("src/lib.ts", "export function libDead() {}\n"),
            ("src/barrel.ts", "export * as ns from \"./lib\";\n"),
        ],
        &Config::default().dead_code,
    );

    assert_eq!(kept_confidence(&findings, "libDead"), None);
}

const MANIFEST_SOURCES: [(&str, &str); 3] = [
    (
        "backend/alerts.ts",
        "import { register } from \"./lib\";\nexport const send = register({ handler: () => 1 });\nexport function formatAlert(e: Error) { return e.message; }\n",
    ),
    (
        "backend/lib.ts",
        "export function register(x: unknown) { return x; }\nexport function libDead() { return 1; }\n",
    ),
    (
        "backend/_gen/api.d.ts",
        "import type * as alerts from \"../alerts.js\";\nimport type * as lib from \"../lib.js\";\nexport declare const api: { alerts: typeof alerts; lib: typeof lib };\n",
    ),
];

#[test]
fn type_only_namespace_manifest_does_not_silence_modules() {
    let findings = scan(&MANIFEST_SOURCES, &Config::default().dead_code);

    assert_eq!(
        kept_confidence(&findings, "formatAlert"),
        Some(Confidence::Medium)
    );
    assert_eq!(
        kept_confidence(&findings, "libDead"),
        Some(Confidence::Medium)
    );
    assert_eq!(kept_confidence(&findings, "send"), Some(Confidence::Medium));
}

#[test]
fn configured_registration_callee_keeps_its_export_live() {
    let mut config = Config::default().dead_code;
    config.entrypoints = vec!["register".to_string()];
    assert_eq!(
        kept_confidence(&scan(&MANIFEST_SOURCES, &config), "send"),
        None
    );
}

#[test]
fn registration_callees_mark_exports_as_entrypoints() {
    let mut config = Config::default().dead_code;
    config.entrypoints = vec!["register".to_string(), "define*".to_string()];
    let findings = scan(
        &[
            (
                "src/alerts.ts",
                "export const send = register({ handler: () => 1 });\nexport const widget = fw.defineWidget({});\nexport const plain = make();\nexport function formatAlert() { return 1; }\n",
            ),
            (
                "src/client.ts",
                "import { formatAlert } from \"./alerts\";\nformatAlert();\n",
            ),
        ],
        &config,
    );

    assert_eq!(kept_confidence(&findings, "send"), None);
    assert_eq!(kept_confidence(&findings, "widget"), None);
    assert_eq!(kept_confidence(&findings, "plain"), Some(Confidence::High));
}

#[test]
fn unused_constants_are_reported_by_default() {
    let findings = scan(
        &[
            (
                "src/limits.ts",
                "export const UNUSED_LIMIT = 1;\nexport const USED_LIMIT = 2;\n",
            ),
            (
                "src/app.ts",
                "import { USED_LIMIT } from \"./limits\";\nconsole.log(USED_LIMIT);\n",
            ),
        ],
        &Config::default().dead_code,
    );

    assert_eq!(
        kept_confidence(&findings, "UNUSED_LIMIT"),
        Some(Confidence::High)
    );
    assert_eq!(kept_confidence(&findings, "USED_LIMIT"), None);
}

#[test]
fn module_typescript_extensions_are_analysed() {
    let findings = scan(
        &[
            (
                "src/b.mts",
                "export function deadMts() {}\nexport function usedMts() {}\n",
            ),
            ("src/c.cts", "export function deadCts() {}\nexport function usedCts() {}\n"),
            (
                "src/a.ts",
                "import { usedMts } from \"./b.mjs\";\nimport { usedCts } from \"./c.cjs\";\nusedMts(usedCts());\n",
            ),
            (
                "src/b.test.mts",
                "import { usedMts } from \"./b.mjs\";\nexport function testHelperDead() {}\n",
            ),
        ],
        &Config::default().dead_code,
    );

    assert_eq!(
        kept_confidence(&findings, "deadMts"),
        Some(Confidence::High)
    );
    assert_eq!(
        kept_confidence(&findings, "deadCts"),
        Some(Confidence::High)
    );
    assert_eq!(kept_confidence(&findings, "usedMts"), None);
    assert_eq!(kept_confidence(&findings, "testHelperDead"), None);
}
