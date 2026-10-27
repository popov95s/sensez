use super::dead_code_tests::{kept_confidence, scan};
use crate::config::model::Config;
use crate::report::{Confidence, DeadCodeFinding};
use crate::spine::parser::SymbolKind;

fn reason<'a>(findings: &'a [DeadCodeFinding], symbol: &str) -> Option<&'a str> {
    findings
        .iter()
        .find(|finding| finding.symbol == symbol)
        .map(|finding| finding.reason.as_str())
}

fn kind_of(findings: &[DeadCodeFinding], symbol: &str) -> Vec<SymbolKind> {
    findings
        .iter()
        .filter(|finding| finding.symbol == symbol)
        .map(|finding| finding.kind)
        .collect()
}

fn playwright_findings() -> Vec<DeadCodeFinding> {
    scan(
        &[
            (
                "e2e/utils/helpers.ts",
                "export function signInAs(role: string) { return role; }\nexport function sharedWait() { return 0; }\nexport function neverUsed() { return 2; }\n",
            ),
            (
                "e2e/utils/only-spec.ts",
                "export function openDrawer() { return 1; }\nexport function reallyDeadInSpecOnlyFile() { return 3; }\n",
            ),
            (
                "e2e/checkout.spec.ts",
                "import { signInAs } from \"./utils/helpers\";\nimport { openDrawer } from \"./utils/only-spec\";\nimport { specOnly } from \"../src/api\";\nsignInAs(\"a\"); openDrawer(); specOnly();\n",
            ),
            (
                "e2e/fixtures.ts",
                "import { sharedWait } from \"./utils/helpers\";\nexport function unloadedFixture() {}\nsharedWait();\n",
            ),
            (
                "src/api.ts",
                "export function specOnly() {}\nexport function appUsed() {}\n",
            ),
            ("src/app.ts", "import { appUsed } from \"./api\";\nappUsed();\n"),
        ],
        &Config::default().dead_code,
    )
}

#[test]
fn spec_importers_keep_playwright_helpers_live() {
    let findings = playwright_findings();
    assert_eq!(kept_confidence(&findings, "signInAs"), None);
    assert_eq!(kept_confidence(&findings, "openDrawer"), None);
}

#[test]
fn unused_playwright_helpers_are_high_confidence() {
    let findings = playwright_findings();
    assert_eq!(
        kept_confidence(&findings, "neverUsed"),
        Some(Confidence::High)
    );
    assert_eq!(
        kept_confidence(&findings, "reallyDeadInSpecOnlyFile"),
        Some(Confidence::High)
    );
}

#[test]
fn playwright_fixture_files_are_not_reported() {
    let findings = playwright_findings();
    assert_eq!(kept_confidence(&findings, "unloadedFixture"), None);
}

#[test]
fn source_used_only_by_specs_is_test_only() {
    let findings = playwright_findings();
    assert_eq!(
        kept_confidence(&findings, "specOnly"),
        Some(Confidence::Medium)
    );
    assert_eq!(reason(&findings, "specOnly"), Some("test_only"));
}

#[test]
fn test_only_usage_is_its_own_reason() {
    let findings = scan(
        &[
            (
                "src/math.ts",
                "export function addUsed() {}\nexport function onlyTested() {}\nexport function zeroRefs() {}\n",
            ),
            ("src/app.ts", "import { addUsed } from \"./math\";\naddUsed();\n"),
            (
                "src/math.test.ts",
                "import { onlyTested } from \"./math\";\nonlyTested();\n",
            ),
            (
                "src/testedOnlyFile.ts",
                "export function helperA() {}\nexport function helperDead() {}\n",
            ),
            (
                "src/tested.test.ts",
                "import { helperA } from \"./testedOnlyFile\";\nhelperA();\n",
            ),
        ],
        &Config::default().dead_code,
    );

    assert_eq!(kept_confidence(&findings, "addUsed"), None);
    assert_eq!(
        kept_confidence(&findings, "onlyTested"),
        Some(Confidence::Medium)
    );
    assert_eq!(reason(&findings, "onlyTested"), Some("test_only"));
    assert_eq!(
        kept_confidence(&findings, "zeroRefs"),
        Some(Confidence::High)
    );
    assert_eq!(reason(&findings, "zeroRefs"), Some(""));
    assert_eq!(
        kind_of(&findings, "helperA, helperDead"),
        vec![SymbolKind::Module]
    );
    assert_eq!(reason(&findings, "helperA, helperDead"), Some("test_only"));
}

#[test]
fn unreachable_modules_report_until_declared_entry_points() {
    let sources = [
        (
            "tools/solo.mjs",
            "export function soloDead() { return 1; }\n",
        ),
        ("src/app.ts", "export function run() {}\n"),
        ("package.json", "{\"main\": \"./dist/app.js\"}"),
    ];
    let mut config = Config::default().dead_code;
    let findings = scan(&sources, &config);

    assert_eq!(
        kept_confidence(&findings, "soloDead"),
        Some(Confidence::Medium)
    );
    assert_eq!(reason(&findings, "soloDead"), Some("unreachable_module"));
    assert_eq!(kept_confidence(&findings, "run"), None);

    config.entry_globs = vec!["**/tools/**".to_string()];
    assert_eq!(kept_confidence(&scan(&sources, &config), "soloDead"), None);

    config.entry_globs.clear();
    config.orphan_modules = false;
    assert_eq!(kept_confidence(&scan(&sources, &config), "soloDead"), None);
}

#[test]
fn package_bin_targets_are_roots() {
    let findings = scan(
        &[
            ("bin/cli.mjs", "export function main2() {}\n"),
            ("package.json", "{\"bin\": {\"tool\": \"./bin/cli.mjs\"}}"),
        ],
        &Config::default().dead_code,
    );

    assert!(findings.is_empty(), "{findings:?}");
}

#[test]
fn entry_point_imports_count_as_usage() {
    let sources = [
        (
            "scripts/seed.ts",
            "import { seedRows } from \"../src/db\";\nseedRows();\n",
        ),
        (
            "src/db.ts",
            "export function seedRows() {}\nexport function dbDead() {}\n",
        ),
    ];
    let mut config = Config::default().dead_code;
    config.entry_globs = vec!["**/scripts/**".to_string()];
    let findings = scan(&sources, &config);

    assert_eq!(kept_confidence(&findings, "seedRows"), None);
    assert_eq!(kept_confidence(&findings, "dbDead"), Some(Confidence::High));
}

#[test]
fn unconsumed_reexports_are_reported_once_at_their_origin() {
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
            ("src/lib/index.ts", "export { helper } from \"../helper\";\n"),
            (
                "src/consumer.ts",
                "import { helper } from \"./public\";\nimport { helper as h2 } from \"./lib\";\nimport { unusedThing } from \"./helper\";\nhelper(h2);\n",
            ),
        ],
        &config,
    );

    assert_eq!(kind_of(&findings, "helper"), vec![]);
    assert_eq!(kind_of(&findings, "orphan"), vec![SymbolKind::Function]);
    assert_eq!(kind_of(&findings, "unusedThing"), vec![SymbolKind::Import]);
}

#[test]
fn reexport_chains_and_stars_forward_to_a_fixed_point() {
    let findings = scan(
        &[
            (
                "src/c.ts",
                "export function viaChain() {}\nexport function viaStar() {}\nexport function chainDead() {}\n",
            ),
            ("src/b.ts", "export { viaChain, chainDead } from \"./c\";\n"),
            ("src/a.ts", "export { viaChain, chainDead } from \"./b\";\n"),
            ("src/star.ts", "export * from \"./c\";\n"),
            (
                "src/app.ts",
                "import { viaChain } from \"./a\";\nimport { viaStar } from \"./star\";\nviaChain(viaStar);\n",
            ),
        ],
        &Config::default().dead_code,
    );

    assert_eq!(kept_confidence(&findings, "viaChain"), None);
    assert_eq!(kept_confidence(&findings, "viaStar"), None);
    assert_eq!(kind_of(&findings, "chainDead"), vec![SymbolKind::Function]);
}

fn hidden_use_findings() -> Vec<DeadCodeFinding> {
    scan(
        &[
            (
                "src/widget.tsx",
                "export default function Widget() { return null; }\nexport function named() {}\nexport function zeroRefs() {}\n",
            ),
            (
                "src/panel.tsx",
                "export default class Panel {}\nexport function panelExtra() {}\n",
            ),
            (
                "src/reporter.ts",
                "export default class Reporter {}\nexport function reporterExtra() {}\n",
            ),
            (
                "src/page.tsx",
                "import { named } from \"./widget\";\nconst LazyPanel = lazy(() => import(\"./panel\"));\nexport function Page() { return [named, LazyPanel]; }\n",
            ),
            (
                "playwright.config.ts",
                "import Reporter from \"./src/reporter\";\nexport default { reporter: [new Reporter()] };\n",
            ),
        ],
        &Config::default().dead_code,
    )
}

#[test]
fn default_export_is_medium_confidence() {
    let findings = hidden_use_findings();
    assert_eq!(
        kept_confidence(&findings, "Widget"),
        Some(Confidence::Medium)
    );
    assert_eq!(
        kept_confidence(&findings, "zeroRefs"),
        Some(Confidence::High)
    );
}

#[test]
fn lazy_import_keeps_default_live_and_lowers_the_rest() {
    let findings = hidden_use_findings();
    assert_eq!(kept_confidence(&findings, "Panel"), None);
    assert_eq!(
        kept_confidence(&findings, "panelExtra"),
        Some(Confidence::Medium)
    );
}

#[test]
fn config_importer_keeps_default_live_and_lowers_the_rest() {
    let findings = hidden_use_findings();
    assert_eq!(kept_confidence(&findings, "Reporter"), None);
    assert_eq!(
        kept_confidence(&findings, "reporterExtra"),
        Some(Confidence::Medium)
    );
}
