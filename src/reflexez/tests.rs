use super::*;
use crate::cli::spec::{ReflexezArgs, RunnerChoice};
use std::fs;
use std::path::Path;
use std::process::Command;

pub(super) struct Project {
    pub(super) root: tempfile::TempDir,
}

pub(super) const VITEST_MANIFEST: &str = r#"{"devDependencies":{"vitest":"4"}}"#;

impl Project {
    pub(super) fn new(test_source: &str) -> Self {
        Self::with_files(&[
            ("src/feature.ts", "export const value = 1;\n"),
            ("src/other.ts", "export const other = 2;\n"),
            ("src/feature.test.ts", test_source),
            (
                "src/other.test.ts",
                "import { other } from './other'; test('other', () => other);\n",
            ),
        ])
    }

    /// A committed project with `files`; a `package.json` using Vitest is added unless given.
    pub(super) fn with_files(files: &[(&str, &str)]) -> Self {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "package.json", VITEST_MANIFEST);
        for (path, source) in files {
            write(root.path(), path, source);
        }
        git(root.path(), &["init"]);
        git(root.path(), &["config", "user.email", "test@example.test"]);
        git(root.path(), &["config", "user.name", "Sensez Test"]);
        git(root.path(), &["add", "."]);
        git(root.path(), &["commit", "-m", "seed"]);
        Self { root }
    }

    pub(super) fn change(&self, path: &str, source: &str) {
        write(self.root.path(), path, source);
    }

    pub(super) fn plan(&self) -> model::ImpactPlan {
        self.plan_with(args())
    }

    pub(super) fn plan_with(&self, args: ReflexezArgs) -> model::ImpactPlan {
        selector::plan(self.root.path(), &args).unwrap()
    }

    pub(super) fn relative(&self, path: &Path) -> String {
        let canonical = fs::canonicalize(self.root.path()).unwrap();
        path.strip_prefix(&canonical)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/")
    }

    pub(super) fn selected(&self, plan: &model::ImpactPlan) -> Vec<String> {
        plan.selected
            .iter()
            .map(|selection| self.relative(&selection.file))
            .collect()
    }
}

#[test]
fn computed_dynamic_import_selects_only_related_test() {
    let project =
        Project::new("const target = './feature'; test('feature', async () => import(target));\n");
    project.change("src/feature.ts", "export const value = 3;\n");

    let plan = project.plan();

    assert!(!plan.full_suite, "{:?}", plan.fallback_reasons);
    assert_eq!(plan.selected.len(), 1);
    assert!(plan.selected[0].file.ends_with("feature.test.ts"));
    assert_eq!(plan.selected[0].reason, model::PlanReason::DynamicImport);
}

#[test]
fn manifest_change_falls_back_to_every_test() {
    let project = Project::new("import './feature'; test('feature', () => 1);\n");
    project.change("package.json", r#"{"devDependencies":{"vitest":"5"}}"#);

    let plan = project.plan();

    assert!(plan.full_suite);
    assert_eq!(plan.selected.len(), 2);
    assert!(plan
        .fallback_reasons
        .iter()
        .any(|reason| reason.contains("manifest")));
}

#[test]
fn opaque_import_in_selected_test_stays_narrow() {
    let project = Project::new(
        "import './feature'; const target = choose(); test('feature', async () => import(target));\n",
    );
    project.change("src/feature.ts", "export const value = 3;\n");

    let plan = project.plan();

    assert!(!plan.full_suite, "{:?}", plan.fallback_reasons);
    assert_eq!(project.selected(&plan), ["src/feature.test.ts"]);
}

#[test]
fn opaque_import_in_unselected_test_selects_that_test() {
    let project =
        Project::new("const target = choose(); test('feature', async () => import (target));\n");
    project.change("src/feature.ts", "export const value = 3;\n");

    let plan = project.plan();

    assert!(!plan.full_suite, "{:?}", plan.fallback_reasons);
    assert_eq!(project.selected(&plan), ["src/feature.test.ts"]);
    assert_eq!(plan.selected[0].reason, model::PlanReason::DynamicImport);
    assert_eq!(plan.unresolved_dynamic_imports, 1);
    assert!(plan.opaque_import_files[0].ends_with("src/feature.test.ts"));
}

#[test]
fn isolated_source_change_selects_no_tests() {
    let project = Project::new("import './feature'; test('feature', () => 1);\n");
    write(
        project.root.path(),
        "src/isolated.ts",
        "export const isolated = 1;\n",
    );
    git(project.root.path(), &["add", "."]);
    git(project.root.path(), &["commit", "-m", "add isolated"]);
    project.change("src/isolated.ts", "export const isolated = 2;\n");

    let plan = project.plan();

    assert!(!plan.full_suite, "{:?}", plan.fallback_reasons);
    assert!(plan.selected.is_empty());
}

#[test]
fn isolated_opaque_import_selects_no_tests() {
    let project = Project::new("import './feature'; test('feature', () => 1);\n");
    write(
        project.root.path(),
        "src/isolated.ts",
        "const t = choose(); import(t); export const y = 1;\n",
    );
    git(project.root.path(), &["add", "."]);
    git(project.root.path(), &["commit", "-m", "add isolated"]);
    project.change(
        "src/isolated.ts",
        "const t = choose(); import(t); export const y = 2;\n",
    );

    let plan = project.plan();

    assert!(!plan.full_suite, "{:?}", plan.fallback_reasons);
    assert!(plan.selected.is_empty());
    assert!(plan.opaque_import_files[0].ends_with("src/isolated.ts"));
}

#[test]
fn isolated_opaque_import_combined_with_affected_file_stays_narrow() {
    let project = Project::new("import './feature'; test('feature', () => 1);\n");
    write(
        project.root.path(),
        "src/isolated.ts",
        "const t = choose(); import(t); export const y = 1;\n",
    );
    git(project.root.path(), &["add", "."]);
    git(project.root.path(), &["commit", "-m", "add isolated"]);
    project.change(
        "src/isolated.ts",
        "const t = choose(); import(t); export const y = 2;\n",
    );
    project.change("src/feature.ts", "export const value = 3;\n");

    let plan = project.plan();

    assert!(!plan.full_suite, "{:?}", plan.fallback_reasons);
    assert_eq!(project.selected(&plan), ["src/feature.test.ts"]);
}

#[test]
fn opaque_import_in_unreached_tool_does_not_widen() {
    let project = Project::with_files(&[
        ("src/feature.ts", "export const value = 1;\n"),
        (
            "src/feature.test.ts",
            "import './feature'; test('f', () => 1);\n",
        ),
        ("src/other.test.ts", "test('o', () => 1);\n"),
        (
            "tools/loader.mjs",
            "export const load = (p) => import(pathToFileURL(p).href);\n",
        ),
    ]);
    project.change("src/feature.ts", "export const value = 2;\n");

    let plan = project.plan();

    assert!(!plan.full_suite, "{:?}", plan.fallback_reasons);
    assert_eq!(project.selected(&plan), ["src/feature.test.ts"]);
    assert!(plan.opaque_import_files[0].ends_with("tools/loader.mjs"));
}

#[test]
fn opaque_import_in_shared_helper_selects_its_dependents() {
    let project = Project::with_files(&[
        ("src/other.ts", "export const other = 1;\n"),
        (
            "src/loaderHelper.ts",
            "export const load = (p: string) => import(p);\n",
        ),
        (
            "src/a.test.ts",
            "import './loaderHelper'; test('a', () => 1);\n",
        ),
        ("src/b.test.ts", "test('b', () => 1);\n"),
    ]);
    project.change("src/other.ts", "export const other = 2;\n");

    let plan = project.plan();

    assert!(!plan.full_suite, "{:?}", plan.fallback_reasons);
    assert_eq!(project.selected(&plan), ["src/a.test.ts"]);
    assert_eq!(plan.selected[0].reason, model::PlanReason::DynamicImport);
}

#[test]
fn only_test_changed_ignores_opaque_imports() {
    let project = Project::new("import './feature'; test('feature', () => 1);\n");
    write(
        project.root.path(),
        "tools/loader.mjs",
        "export const load = (p) => import(p);\n",
    );
    git(project.root.path(), &["add", "."]);
    git(project.root.path(), &["commit", "-m", "add loader"]);
    project.change(
        "src/feature.test.ts",
        "import './feature'; test('feature', () => 2);\n",
    );

    let plan = project.plan();

    assert!(!plan.full_suite, "{:?}", plan.fallback_reasons);
    assert_eq!(project.selected(&plan), ["src/feature.test.ts"]);
}

#[test]
fn strict_dynamic_widens_on_any_opaque_import() {
    let project = Project::new("import './feature'; test('feature', () => 1);\n");
    write(
        project.root.path(),
        "tools/loader.mjs",
        "export const load = (p) => import(p);\n",
    );
    git(project.root.path(), &["add", "."]);
    git(project.root.path(), &["commit", "-m", "add loader"]);
    project.change("src/feature.ts", "export const value = 3;\n");

    let plan = project.plan_with(ReflexezArgs {
        strict_dynamic: true,
        ..args()
    });

    assert!(plan.full_suite);
}

#[test]
fn array_glob_is_resolved() {
    let project = Project::with_files(&[
        (
            "src/registry.test.ts",
            "const handlers = import.meta.glob(['./handlers/*.ts', '!./handlers/skip.ts']);\ntest('r', () => handlers);\n",
        ),
        ("src/handlers/alpha.ts", "export const alpha = 1;\n"),
        ("src/handlers/skip.ts", "export const skip = 1;\n"),
    ]);
    project.change("src/handlers/alpha.ts", "export const alpha = 2;\n");

    let plan = project.plan();

    assert_eq!(plan.unresolved_dynamic_imports, 0);
    assert!(!plan.full_suite, "{:?}", plan.fallback_reasons);
    assert_eq!(project.selected(&plan), ["src/registry.test.ts"]);

    project.change("src/handlers/alpha.ts", "export const alpha = 1;\n");
    project.change("src/handlers/skip.ts", "export const skip = 2;\n");
    assert!(project.plan().selected.is_empty());
}

pub(super) fn args() -> ReflexezArgs {
    ReflexezArgs {
        path: None,
        base: None,
        staged: false,
        changed_files: Vec::new(),
        plan: true,
        json: false,
        full: false,
        strict_dynamic: false,
        runner: RunnerChoice::Auto,
        runner_args: Vec::new(),
    }
}

pub(super) fn write(root: &Path, relative: &str, source: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, source).unwrap();
}

pub(super) fn git(root: &Path, arguments: &[&str]) {
    let status = Command::new("git")
        .args(arguments)
        .current_dir(root)
        .status()
        .unwrap();
    assert!(status.success());
}
