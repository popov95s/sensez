use super::*;

#[test]
fn vitest_projects_matching_nothing_keep_the_config_as_one_project() {
    let fixture = Fixture::new(&[(
        "vitest.config.ts",
        r#"export default { test: { projects: ["packages/*"] } };"#,
    )]);

    let config = fixture.load(RunnerKind::Vitest);

    assert!(config.is_test(&fixture.path("src/a.test.ts")));
}

#[test]
fn jest_projects_matching_nothing_keep_the_config_as_one_project() {
    let fixture = Fixture::new(&[(
        "jest.config.js",
        r#"module.exports = { projects: ["<rootDir>/packages/*"] };"#,
    )]);

    let config = fixture.load(RunnerKind::Jest);

    assert!(config.is_test(&fixture.path("src/a.test.ts")));
}
