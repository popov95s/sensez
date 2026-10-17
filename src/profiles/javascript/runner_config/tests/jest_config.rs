use super::*;

#[test]
fn reads_jest_config_keys() {
    let fixture = Fixture::new(&[
        ("setup.js", ""),
        (
            "jest.config.js",
            r#"module.exports = {
  roots: ["<rootDir>/src"],
  testMatch: ["**/*.spec.tsx", "**/__tests__/**/*.ts"],
  testPathIgnorePatterns: ["/node_modules/", "/fixtures/"],
  setupFilesAfterEnv: ["<rootDir>/setup.js", "jest-extended"],
};"#,
        ),
    ]);

    let config = fixture.jest();

    assert!(config.warnings.is_empty(), "{:?}", config.warnings);
    assert_eq!(
        fixture.tests(&config, &CANDIDATES),
        ["src/__tests__/kit.testkit.ts", "src/a.spec.tsx"]
    );
    assert!(!config.is_test(&fixture.path("src/fixtures/a.spec.tsx")));
    assert_eq!(fixture.setup(&config), [PathBuf::from("setup.js")]);
}

#[test]
fn reads_jest_package_json_key_with_test_regex() {
    let fixture = Fixture::new(&[(
        "package.json",
        r#"{"jest": {"rootDir": "src", "testRegex": "\\.test\\.ts$"}}"#,
    )]);

    let config = fixture.jest();

    assert!(config.warnings.is_empty(), "{:?}", config.warnings);
    assert_eq!(
        fixture.tests(&config, &CANDIDATES),
        ["src/a.test.ts", "src/a.llm.test.ts"]
    );
}

#[test]
fn jest_defaults_keep_tests_dirs() {
    let fixture = Fixture::new(&[(
        "jest.config.ts",
        r#"import type { Config } from "jest";
const config: Config = { verbose: true };
export default config;"#,
    )]);

    let config = fixture.jest();

    assert!(config.warnings.is_empty(), "{:?}", config.warnings);
    assert!(config.is_test(&fixture.path("src/__tests__/thing.js")));
    assert!(!config.is_test(&fixture.path("node_modules/x/a.test.js")));
    assert!(!config.is_test(&fixture.path("src/helper.ts")));
}
