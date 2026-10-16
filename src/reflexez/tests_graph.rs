use super::model::PlanReason;
use super::tests::Project;

#[test]
fn changed_css_module_selects_importing_test() {
    let project = Project::with_files(&[
        ("src/widget.module.css", ".a {}\n"),
        (
            "src/widget.test.ts",
            "import styles from './widget.module.css'; test('w', () => styles);\n",
        ),
        ("src/other.test.ts", "test('o', () => 1);\n"),
    ]);
    project.change("src/widget.module.css", ".a { color: red }\n");

    let plan = project.plan();

    assert!(!plan.full_suite, "{:?}", plan.fallback_reasons);
    assert_eq!(project.selected(&plan), ["src/widget.test.ts"]);
    assert_eq!(plan.selected[0].reason, PlanReason::DirectDependency);
}

#[test]
fn changed_json_fixture_selects_transitive_test() {
    let project = Project::with_files(&[
        ("src/fixture.json", "{}\n"),
        (
            "src/loader.ts",
            "import data from './fixture.json'; export const load = () => data;\n",
        ),
        (
            "src/loader.test.ts",
            "import { load } from './loader'; test('l', load);\n",
        ),
    ]);
    project.change("src/fixture.json", "{\"a\": 1}\n");

    let plan = project.plan();

    assert_eq!(project.selected(&plan), ["src/loader.test.ts"]);
    assert_eq!(plan.selected[0].distance, 2);
}

#[test]
fn aliased_asset_import() {
    let project = Project::with_files(&[
        (
            "tsconfig.json",
            r#"{"compilerOptions": {"paths": {"@/*": ["src/*"]}}}"#,
        ),
        ("src/assets/logo.svg", "<svg/>\n"),
        (
            "src/logo.test.ts",
            "import logo from '@/assets/logo.svg?url'; test('l', () => logo);\n",
        ),
    ]);
    project.change("src/assets/logo.svg", "<svg></svg>\n");

    let plan = project.plan();

    assert!(!plan.full_suite, "{:?}", plan.fallback_reasons);
    assert_eq!(project.selected(&plan), ["src/logo.test.ts"]);
}

#[test]
fn unreferenced_asset_selects_nothing_but_is_reported() {
    let project = Project::with_files(&[
        ("src/unused.css", ".a {}\n"),
        ("src/a.test.ts", "test('a', () => 1);\n"),
    ]);
    project.change("src/unused.css", ".b {}\n");

    let plan = project.plan();

    assert!(!plan.full_suite, "{:?}", plan.fallback_reasons);
    assert!(plan.selected.is_empty());
    assert_eq!(plan.unmapped_changes.len(), 1);
    assert_eq!(
        project.relative(&plan.unmapped_changes[0]),
        "src/unused.css"
    );
}

fn glob_registry(pattern: &str) -> Project {
    Project::with_files(&[
        (
            "tsconfig.json",
            r#"{"compilerOptions": {"paths": {"@/*": ["src/*"]}}}"#,
        ),
        (
            "src/registry.ts",
            &format!("export const handlers = import.meta.glob('{pattern}', {{ eager: true }});\n"),
        ),
        (
            "src/registry.test.ts",
            "import { handlers } from './registry'; test('r', () => handlers);\n",
        ),
        ("src/handlers/alpha.ts", "export const alpha = 1;\n"),
        (
            "src/handlers/beta.ts",
            "import { help } from '../helper'; export const beta = help;\n",
        ),
        ("src/helper.ts", "export const help = 1;\n"),
        ("src/unrelated.test.ts", "test('u', () => 1);\n"),
    ])
}

#[test]
fn glob_registry_reached_through_static_import() {
    let project = glob_registry("./handlers/*.ts");
    project.change("src/handlers/alpha.ts", "export const alpha = 2;\n");

    let plan = project.plan();

    assert_eq!(project.selected(&plan), ["src/registry.test.ts"]);
    assert_eq!(plan.selected[0].reason, PlanReason::DynamicImport);
    assert_eq!(plan.selected[0].distance, 2);
}

#[test]
fn static_dependency_of_globbed_module() {
    let project = glob_registry("./handlers/*.ts");
    project.change("src/helper.ts", "export const help = 2;\n");

    let plan = project.plan();

    assert_eq!(project.selected(&plan), ["src/registry.test.ts"]);
    assert_eq!(plan.selected[0].distance, 3);
}

#[test]
fn alias_glob() {
    let project = glob_registry("@/handlers/*.ts");
    project.change("src/handlers/alpha.ts", "export const alpha = 2;\n");

    let plan = project.plan();

    assert_eq!(project.selected(&plan), ["src/registry.test.ts"]);
}

#[test]
fn generated_source_change_selects_importer() {
    let project = Project::with_files(&[
        ("src/generated/api.ts", "export const api = 1;\n"),
        (
            "src/api.test.ts",
            "import { api } from './generated/api'; test('a', () => api);\n",
        ),
    ]);
    project.change("src/generated/api.ts", "export const api = 2;\n");

    let plan = project.plan();

    assert_eq!(project.selected(&plan), ["src/api.test.ts"]);
}

#[test]
fn vendor_test_is_discovered_when_runner_includes_it() {
    let project = Project::with_files(&[
        ("src/vendor/lib/lib.test.ts", "test('v', () => 1);\n"),
        ("src/a.test.ts", "test('a', () => 1);\n"),
    ]);

    let plan = project.plan();

    assert_eq!(plan.discovered_tests, 2);
}

#[test]
fn unmapped_existing_source_is_reported() {
    let project = Project::with_files(&[
        ("src/a.test.ts", "test('a', () => 1);\n"),
        ("target/gen.ts", "export const gen = 1;\n"),
    ]);
    project.change("target/gen.ts", "export const gen = 2;\n");

    let plan = project.plan();

    assert!(!plan.full_suite, "{:?}", plan.fallback_reasons);
    assert_eq!(plan.unmapped_changes.len(), 1);
    assert_eq!(project.relative(&plan.unmapped_changes[0]), "target/gen.ts");
}
