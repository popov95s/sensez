use super::dead_code_tests::{kept_confidence, scan, with_variables};
use crate::config::model::Config;
use crate::report::{Confidence, DeadCodeFinding};
use crate::spine::parser::SymbolKind;

fn in_file<'a>(findings: &'a [DeadCodeFinding], suffix: &str) -> Vec<&'a DeadCodeFinding> {
    findings
        .iter()
        .filter(|finding| finding.file.to_string_lossy().contains(suffix))
        .collect()
}

fn kinds_of(findings: &[DeadCodeFinding], symbol: &str) -> Vec<SymbolKind> {
    findings
        .iter()
        .filter(|finding| finding.symbol == symbol)
        .map(|finding| finding.kind)
        .collect()
}

#[test]
fn directory_loaded_through_a_type_manifest_is_runtime_loaded() {
    let handler = "export const run = define({});\nexport function formatRow() {}\n";
    let findings = scan(
        &[
            ("server/jobs/a.ts", handler),
            ("server/jobs/b.ts", handler),
            ("server/jobs/c.ts", handler),
            ("server/jobs/d.ts", handler),
            ("server/jobs/e.ts", handler),
            (
                "server/_gen/api.d.ts",
                "import type * as a from \"../jobs/a.js\";\nimport type * as b from \"../jobs/b.js\";\nimport type * as c from \"../jobs/c.js\";\nimport type * as d from \"../jobs/d.js\";\nimport type * as e from \"../jobs/e.js\";\nexport declare const api: { a: typeof a; b: typeof b; c: typeof c; d: typeof d; e: typeof e };\n",
            ),
        ],
        &Config::default().dead_code,
    );

    assert!(
        in_file(&findings, "server/jobs/").is_empty(),
        "{findings:?}"
    );
}

#[test]
fn barrel_names_consumed_through_a_directory_import_are_live() {
    let findings = scan(
        &[
            (
                "src/billing/types/validators.ts",
                "export const invoiceShape = 1;\nexport const refundShape = 2;\n",
            ),
            (
                "src/billing/types/index.ts",
                "export { invoiceShape, refundShape } from \"./validators\";\n",
            ),
            (
                "src/billing/service.ts",
                "import { invoiceShape } from \"./types\";\nexport function charge() { return invoiceShape; }\n",
            ),
            (
                "src/app.ts",
                "import { charge } from \"./billing/service\";\ncharge();\n",
            ),
        ],
        &with_variables(),
    );

    assert_eq!(kept_confidence(&findings, "invoiceShape"), None);
    assert_eq!(
        kinds_of(&findings, "refundShape"),
        vec![SymbolKind::Variable]
    );
}

#[test]
fn barrel_entries_bypassed_by_direct_importers_are_not_reported() {
    let findings = scan(
        &[
            (
                "src/shapes/money.ts",
                "export const currencyCode = 'EUR';\nexport type Cents = number;\nexport type Unpriced = string;\nexport function roundCents(c: Cents) { return c; }\n",
            ),
            (
                "src/shapes/index.ts",
                "export { currencyCode, roundCents, type Cents, type Unpriced } from \"./money\";\n",
            ),
            (
                "src/app.ts",
                "import { currencyCode, type Cents } from \"./shapes/money\";\nimport { roundCents } from \"./shapes\";\nconst c: Cents = 1;\nconsole.log(currencyCode, roundCents(c));\n",
            ),
        ],
        &Config::default().dead_code,
    );

    assert_eq!(kept_confidence(&findings, "currencyCode"), None);
    assert_eq!(kept_confidence(&findings, "Cents"), None);
    assert!(kinds_of(&findings, "Unpriced").is_empty());
}

#[test]
fn unimported_module_is_reported_once() {
    let findings = scan(
        &[
            (
                "tools/legacy.ts",
                "const local = 0;\nexport function first() { return local; }\nexport function second() {}\nexport const third = 1;\n",
            ),
            (
                "src/app.ts",
                "import { used } from \"./lib\";\nused();\n",
            ),
            ("src/lib.ts", "export function used() {}\n"),
        ],
        &with_variables(),
    );

    let legacy = in_file(&findings, "legacy.ts");
    assert_eq!(legacy.len(), 1, "{findings:?}");
    assert_eq!(legacy[0].kind, SymbolKind::Module);
    assert_eq!(legacy[0].symbol, "first, second, third");
    assert_eq!(legacy[0].reason, "unreachable_module");
    assert_eq!(legacy[0].confidence, Confidence::Medium);
    assert_eq!(legacy[0].line, 2);
}

#[test]
fn imports_of_unimported_modules_count_as_usage() {
    let findings = scan(
        &[
            (
                "scripts/backfill.ts",
                "import { migrateRows } from \"../src/db\";\nmigrateRows();\n",
            ),
            (
                "src/db.ts",
                "export function migrateRows() {}\nexport function dropRows() {}\n",
            ),
        ],
        &Config::default().dead_code,
    );

    assert_eq!(kept_confidence(&findings, "migrateRows"), None);
    assert_eq!(
        kept_confidence(&findings, "dropRows"),
        Some(Confidence::High)
    );
}

fn runtime_loaded_sources() -> Vec<(&'static str, &'static str)> {
    let handler = "import { normalize } from \"../../lib/text\";\nexport const handler = () => normalize('x');\nexport const settings = {};\n";
    vec![
        ("api/routes/createUser.ts", handler),
        ("api/routes/deleteUser.ts", handler),
        ("api/routes/listUsers.ts", handler),
        ("api/routes/renameUser.ts", handler),
        ("api/routes/mergeUser.ts", handler),
        (
            "lib/text.ts",
            "export function normalize(s: string) { return s; }\nexport function shout(s: string) { return s; }\n",
        ),
    ]
}

#[test]
fn directory_of_unimported_modules_is_runtime_loaded() {
    let findings = scan(&runtime_loaded_sources(), &Config::default().dead_code);

    assert!(in_file(&findings, "api/routes/").is_empty(), "{findings:?}");
    assert_eq!(kept_confidence(&findings, "normalize"), None);
    assert_eq!(kept_confidence(&findings, "shout"), Some(Confidence::High));
}

#[test]
fn runtime_loaded_directories_can_be_turned_off() {
    let mut config = Config::default().dead_code;
    config.unimported_dir_share = 0;
    let findings = scan(&runtime_loaded_sources(), &config);

    let routes: Vec<_> = in_file(&findings, "api/routes/")
        .into_iter()
        .map(|finding| finding.kind)
        .collect();
    assert_eq!(routes, vec![SymbolKind::Module; 5], "{findings:?}");
}

#[test]
fn builder_chain_callees_register_exports() {
    let mut config = Config::default().dead_code;
    config.registration_decorators = vec!["guarded".to_string()];
    let findings = scan(
        &[
            (
                "src/jobs.ts",
                "export const nightly = base.guarded(\"admin\")({ run() {} });\nexport function schedule() {}\n",
            ),
            ("src/main.ts", "import { schedule } from \"./jobs\";\nschedule();\n"),
        ],
        &config,
    );

    assert_eq!(kept_confidence(&findings, "nightly"), None, "{findings:?}");
}

fn manifest_sources(handlers: usize) -> Vec<(String, String)> {
    let mut sources: Vec<(String, String)> = (0..handlers)
        .map(|n| {
            (
                format!("server/mixed/h{n}.ts"),
                "export const job = task({});\nexport const shared = 1;\nexport function leftover() {}\n".to_string(),
            )
        })
        .collect();
    let imports: String = (0..handlers)
        .map(|n| format!("import type * as h{n} from \"../mixed/h{n}.js\";\n"))
        .collect();
    let fields: String = (0..handlers)
        .map(|n| format!("h{n}: typeof h{n}; "))
        .collect();
    sources.push((
        "server/_gen/api.d.ts".to_string(),
        format!("{imports}export declare const api: {{ {fields}}};\n"),
    ));
    let users: String = (0..handlers)
        .map(|n| {
            format!("import {{ shared as s{n} }} from \"./mixed/h{n}\";\nconsole.log(s{n});\n")
        })
        .collect();
    sources.push(("server/app.ts".to_string(), users));
    sources
}

#[test]
fn callees_repeated_across_a_type_manifest_register_exports() {
    let sources = manifest_sources(5);
    let borrowed: Vec<(&str, &str)> = sources
        .iter()
        .map(|(p, s)| (p.as_str(), s.as_str()))
        .collect();
    let findings = scan(&borrowed, &Config::default().dead_code);

    assert_eq!(kept_confidence(&findings, "job"), None);
    assert_eq!(
        kept_confidence(&findings, "leftover"),
        Some(Confidence::Medium)
    );
}

#[test]
fn barrel_entries_relayed_from_a_live_definition_are_not_reported() {
    let findings = scan(
        &[
            ("src/core/ids.ts", "export const idShape = 1;\n"),
            (
                "src/feature/types/validators.ts",
                "import { idShape } from \"../../core/ids\";\nexport const rowShape = 2;\nexport { idShape };\n",
            ),
            (
                "src/feature/types/index.ts",
                "export { idShape, rowShape } from \"./validators\";\n",
            ),
            (
                "src/app.ts",
                "import { idShape } from \"./core/ids\";\nimport { rowShape } from \"./feature/types\";\nconsole.log(idShape, rowShape);\n",
            ),
        ],
        &Config::default().dead_code,
    );

    assert!(findings.is_empty(), "{findings:?}");
}

#[test]
fn type_only_reexports_are_not_reported() {
    let findings = scan(
        &[
            (
                "src/billing/validators.ts",
                "export type Invoice = { total: number };\nexport interface Refund { amount: number }\nexport function chargeAll() {}\n",
            ),
            (
                "src/billing/types.ts",
                "export { Invoice, Refund, chargeAll } from \"./validators\";\n",
            ),
            (
                "src/app.ts",
                "import { chargeAll } from \"./billing/types\";\nchargeAll();\n",
            ),
        ],
        &Config::default().dead_code,
    );

    assert!(kinds_of(&findings, "Invoice").is_empty());
    assert!(kinds_of(&findings, "Refund").is_empty());
}
