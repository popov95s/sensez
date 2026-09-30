use super::tests::{git, write, Project};

fn commit_config(project: &Project, config: &str) {
    write(project.root.path(), "sensez.toml", config);
    git(project.root.path(), &["add", "."]);
    git(project.root.path(), &["commit", "-m", "config"]);
}

#[test]
fn unrelated_config_typo_is_a_plan_warning_not_a_fallback() {
    let project = Project::new("import './feature'; test('feature', () => 1);\n");
    commit_config(&project, "[duplication]\ntreshold = 5\n");
    project.change("src/feature.ts", "export const value = 3;\n");

    let plan = project.plan();

    assert!(!plan.full_suite, "{:?}", plan.fallback_reasons);
    assert_eq!(
        plan.settings_warnings,
        vec![
            "sensez.toml:2:1: unknown key `duplication.treshold` ignored (did you mean `threshold`?)"
        ]
    );
}

#[test]
fn invalid_config_is_a_plan_warning() {
    let project = Project::new("import './feature'; test('feature', () => 1);\n");
    commit_config(&project, "[smells]\nmax_cyclomatc = 5\n");
    project.change("src/feature.ts", "export const value = 3;\n");

    let plan = project.plan();

    assert!(plan.settings_warnings[0].starts_with("sensez.toml:2:1:"));
}
