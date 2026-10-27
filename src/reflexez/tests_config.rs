use super::model::{RunMode, RunnerKind};
use super::tests::{args, Project};
use crate::cli::spec::ReflexezArgs;

fn setup_project(key: &str) -> Project {
    Project::with_files(&[
        (
            "vitest.config.ts",
            &format!(
                "export default defineConfig({{ test: {{ {key}: ['./src/testSetup.ts'] }} }});\n"
            ),
        ),
        (
            "src/testSetup.ts",
            "import { reset } from './resetHelper'; reset();\n",
        ),
        ("src/resetHelper.ts", "export const reset = () => 1;\n"),
        ("src/feature.ts", "export const value = 1;\n"),
        ("src/unrelated.ts", "export const unrelated = 1;\n"),
        (
            "src/feature.test.ts",
            "import './feature'; test('f', () => 1);\n",
        ),
        ("src/other.test.ts", "test('o', () => 1);\n"),
    ])
}

#[test]
fn setup_file_or_its_import_change_forces_full_run() {
    for key in ["setupFiles", "globalSetup"] {
        for changed in ["src/testSetup.ts", "src/resetHelper.ts"] {
            let project = setup_project(key);
            project.change(changed, "export const reset = () => 2;\n");

            let plan = project.plan();

            assert!(plan.full_suite, "{key} {changed}");
            assert!(
                plan.fallback_reasons
                    .iter()
                    .any(|reason| reason.contains("test setup src/testSetup.ts")),
                "{:?}",
                plan.fallback_reasons
            );
        }
    }
}

#[test]
fn unrelated_source_change_ignores_setup() {
    let project = setup_project("setupFiles");
    project.change("src/unrelated.ts", "export const unrelated = 2;\n");

    let plan = project.plan();

    assert!(!plan.full_suite, "{:?}", plan.fallback_reasons);
}

#[test]
fn nonliteral_setup_warns_and_widens_only_when_strict() {
    let project = Project::with_files(&[
        (
            "vitest.config.ts",
            "import { computeSetup } from './tools/setup';\nexport default defineConfig({ test: { setupFiles: computeSetup() } });\n",
        ),
        ("src/feature.ts", "export const value = 1;\n"),
        ("src/feature.test.ts", "import './feature'; test('f', () => 1);\n"),
    ]);
    project.change("src/feature.ts", "export const value = 2;\n");

    let plan = project.plan();

    assert!(!plan.full_suite, "{:?}", plan.fallback_reasons);
    assert_eq!(plan.config_warnings.len(), 1);
    assert_eq!(plan.config_warnings[0].key, "setupFiles");
    assert!(plan.config_warnings[0].file.ends_with("vitest.config.ts"));

    let strict = project.plan_with(ReflexezArgs {
        strict_dynamic: true,
        ..args()
    });
    assert!(strict.full_suite);
}

#[test]
fn vitest_include_exclude_respected() {
    let project = Project::with_files(&[
        (
            "vitest.config.ts",
            "export default defineConfig({ test: { include: ['**/*.test.ts'], exclude: ['**/*.llm.test.ts'] } });\n",
        ),
        ("src/helper.ts", "export const help = 1;\n"),
        ("src/__tests__/kit.testkit.ts", "export const kit = 1;\n"),
        (
            "src/widget.llm.test.ts",
            "import './helper'; test('l', () => 1);\n",
        ),
        ("src/widget.test.ts", "test('w', () => 1);\n"),
    ]);
    project.change("src/helper.ts", "export const help = 2;\n");

    let plan = project.plan();

    assert_eq!(plan.discovered_tests, 1);
    assert!(plan.selected.is_empty());
}

#[test]
fn vitest_default_ignores_plain_files_in_tests_dirs() {
    let project = Project::with_files(&[
        ("src/__tests__/helper.ts", "export const help = 1;\n"),
        ("src/a.test.ts", "test('a', () => 1);\n"),
    ]);

    assert_eq!(project.plan().discovered_tests, 1);
}

#[test]
fn jest_default_keeps_tests_dirs() {
    let project = Project::with_files(&[
        ("package.json", r#"{"devDependencies":{"jest":"29"}}"#),
        ("src/__tests__/thing.js", "test('t', () => 1);\n"),
    ]);

    let plan = project.plan();

    assert_eq!(plan.discovered_tests, 1);
}

#[test]
fn full_plan_passes_no_paths() {
    let project = Project::with_files(&[("src/a.test.ts", "test('a', () => 1);\n")]);

    let plan = project.plan_with(ReflexezArgs {
        full: true,
        ..args()
    });

    assert_eq!(plan.runners.len(), 1);
    assert!(plan
        .runners
        .iter()
        .all(|runner| runner.mode == RunMode::Full && runner.tests.is_empty()));
    assert_eq!(plan.runners[0].prefix_args, ["run"]);
}

#[test]
fn safety_fallback_passes_no_paths() {
    let project = Project::with_files(&[("src/a.test.ts", "test('a', () => 1);\n")]);
    project.change("package.json", r#"{"devDependencies":{"vitest":"5"}}"#);

    let plan = project.plan();

    assert!(plan.full_suite);
    assert!(plan
        .runners
        .iter()
        .all(|runner| runner.mode == RunMode::Full && runner.tests.is_empty()));
}

#[test]
fn jest_full_drops_run_tests_by_path() {
    let project = Project::with_files(&[
        ("package.json", r#"{"devDependencies":{"jest":"29"}}"#),
        ("src/a.test.js", "test('a', () => 1);\n"),
    ]);

    let plan = project.plan_with(ReflexezArgs {
        full: true,
        ..args()
    });

    assert_eq!(plan.runners[0].kind, RunnerKind::Jest);
    assert!(plan.runners[0].prefix_args.is_empty());
}

#[test]
fn narrowed_plan_passes_with_no_tests_flag() {
    let project = Project::with_files(&[
        ("src/feature.ts", "export const value = 1;\n"),
        (
            "src/feature.test.ts",
            "import './feature'; test('f', () => 1);\n",
        ),
    ]);
    project.change("src/feature.ts", "export const value = 2;\n");

    let plan = project.plan();

    assert_eq!(plan.runners[0].mode, RunMode::Narrowed);
    assert_eq!(plan.runners[0].prefix_args, ["run", "--passWithNoTests"]);
}

#[test]
fn terminal_output_uses_snake_case_reasons() {
    let project = Project::with_files(&[
        ("src/feature.ts", "export const value = 1;\n"),
        (
            "src/feature.test.ts",
            "import './feature'; test('f', () => 1);\n",
        ),
    ]);
    project.change("src/feature.ts", "export const value = 2;\n");

    let output = super::render::terminal(&project.plan());

    assert!(output.contains("direct_dependency"), "{output}");
}

#[test]
fn terminal_output_is_relative_to_the_repository_when_run_from_a_subdirectory() {
    let project = Project::with_files(&[
        ("web/feature.ts", "export const value = 1;\n"),
        (
            "web/feature.test.ts",
            "import './feature'; test('f', () => 1);\n",
        ),
    ]);
    let args = ReflexezArgs {
        path: Some(project.root.path().join("web")),
        full: true,
        ..args()
    };

    let plan = super::selector::plan(&project.root.path().join("web"), &args).unwrap();
    let output = super::render::terminal(&plan);

    assert!(!output.contains(".//"), "{output}");
    assert!(output.contains("    web/feature.test.ts"), "{output}");
}
