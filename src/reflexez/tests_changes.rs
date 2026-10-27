use super::tests::{args, git, write, Project, VITEST_MANIFEST};

fn math_project() -> Project {
    Project::with_files(&[
        ("README.md", "# demo\n"),
        ("src/math.ts", "export const add = 1;\n"),
        (
            "src/math.test.ts",
            "import { add } from './math'; test('add', () => add);\n",
        ),
        ("src/other.ts", "export const other = 2;\n"),
        (
            "src/other.test.ts",
            "import { other } from './other'; test('other', () => other);\n",
        ),
    ])
}

#[test]
fn deleted_readme_keeps_the_plan_narrow() {
    let project = math_project();
    git(project.root.path(), &["rm", "-q", "README.md"]);

    let plan = project.plan();

    assert!(!plan.full_suite, "{:?}", plan.fallback_reasons);
}

#[test]
fn renamed_source_is_treated_as_a_deletion() {
    let project = math_project();
    git(project.root.path(), &["mv", "src/math.ts", "src/arith.ts"]);

    let plan = project.plan();

    assert!(plan.full_suite);
    assert!(
        plan.fallback_reasons
            .iter()
            .any(|reason| reason.contains("deleted")),
        "{:?}",
        plan.fallback_reasons
    );
}

#[test]
fn staged_rename_is_treated_as_a_deletion() {
    let project = math_project();
    git(project.root.path(), &["mv", "src/math.ts", "src/arith.ts"]);

    let plan = project.plan_with(crate::cli::spec::ReflexezArgs {
        staged: true,
        ..args()
    });

    assert!(plan.full_suite, "{:?}", plan.fallback_reasons);
}

#[test]
fn case_only_rename_is_treated_as_a_deletion() {
    let project = math_project();
    git(project.root.path(), &["mv", "src/math.ts", "src/Math.ts"]);

    let plan = project.plan();

    assert!(plan.full_suite, "{:?}", plan.fallback_reasons);
}

#[test]
fn repository_without_commits_plans_every_file_as_changed() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "package.json", VITEST_MANIFEST);
    write(root.path(), "a.ts", "export const a = 1;\n");
    write(
        root.path(),
        "a.test.ts",
        "import { a } from './a'; test('a', () => a);\n",
    );
    git(root.path(), &["init", "-q"]);

    let plan = super::selector::plan(root.path(), &args()).unwrap();

    assert_eq!(plan.selected.len(), 1);
}
