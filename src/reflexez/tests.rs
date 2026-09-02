use super::*;
use crate::cli::spec::{ReflexezArgs, RunnerChoice};
use std::fs;
use std::path::Path;
use std::process::Command;

struct Project {
    root: tempfile::TempDir,
}

impl Project {
    fn new(test_source: &str) -> Self {
        let root = tempfile::tempdir().unwrap();
        write(
            root.path(),
            "package.json",
            r#"{"devDependencies":{"vitest":"4"}}"#,
        );
        write(root.path(), "src/feature.ts", "export const value = 1;\n");
        write(root.path(), "src/other.ts", "export const other = 2;\n");
        write(root.path(), "src/feature.test.ts", test_source);
        write(
            root.path(),
            "src/other.test.ts",
            "import { other } from './other'; test('other', () => other);\n",
        );
        git(root.path(), &["init"]);
        git(root.path(), &["config", "user.email", "test@example.test"]);
        git(root.path(), &["config", "user.name", "Sensez Test"]);
        git(root.path(), &["add", "."]);
        git(root.path(), &["commit", "-m", "seed"]);
        Self { root }
    }

    fn change(&self, path: &str, source: &str) {
        write(self.root.path(), path, source);
    }

    fn plan(&self) -> model::ImpactPlan {
        selector::plan(self.root.path(), &args()).unwrap()
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
fn opaque_import_in_selected_test_forces_safe_fallback() {
    let project = Project::new(
        "import './feature'; const target = choose(); test('feature', async () => import(target));\n",
    );
    project.change("src/feature.ts", "export const value = 3;\n");

    let plan = project.plan();

    assert!(plan.full_suite);
    assert_eq!(plan.selected.len(), 2);
    assert!(plan
        .fallback_reasons
        .iter()
        .any(|reason| reason.contains("dynamic import")));
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

fn args() -> ReflexezArgs {
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

fn write(root: &Path, relative: &str, source: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, source).unwrap();
}

fn git(root: &Path, arguments: &[&str]) {
    let status = Command::new("git")
        .args(arguments)
        .current_dir(root)
        .status()
        .unwrap();
    assert!(status.success());
}
