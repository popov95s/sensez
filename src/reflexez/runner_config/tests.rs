mod empty_projects;
mod jest;

use super::*;
use std::fs;

struct Fixture {
    root: tempfile::TempDir,
}

impl Fixture {
    fn new(files: &[(&str, &str)]) -> Self {
        let root = tempfile::tempdir().unwrap();
        for (path, source) in files {
            let path = root.path().join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, source).unwrap();
        }
        Self { root }
    }

    fn load(&self, runner: RunnerKind) -> RunnerConfig {
        RunnerConfig::load(self.root.path(), runner)
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root.path().join(relative)
    }

    fn tests(&self, config: &RunnerConfig, candidates: &[&str]) -> Vec<String> {
        candidates
            .iter()
            .filter(|candidate| config.is_test(&self.path(candidate)))
            .map(|candidate| candidate.to_string())
            .collect()
    }

    fn setup(&self, config: &RunnerConfig) -> Vec<PathBuf> {
        config
            .setup_files
            .iter()
            .map(|setup| {
                setup
                    .path
                    .strip_prefix(self.root.path())
                    .unwrap()
                    .to_path_buf()
            })
            .collect()
    }
}

const CANDIDATES: [&str; 5] = [
    "src/a.test.ts",
    "src/a.llm.test.ts",
    "src/__tests__/kit.testkit.ts",
    "src/a.spec.tsx",
    "e2e/a.spec.ts",
];

#[test]
fn reads_define_config_literal_with_string_setup() {
    let fixture = Fixture::new(&[
        ("src/setup.ts", ""),
        (
            "vitest.config.ts",
            r#"import { defineConfig } from "vitest/config";
export default defineConfig({
  test: {
    include: ["src/**/*.test.ts"],
    exclude: ["**/*.llm.test.ts"],
    setupFiles: "./src/setup.ts",
  },
});"#,
        ),
    ]);

    let config = fixture.load(RunnerKind::Vitest);

    assert!(config.warnings.is_empty(), "{:?}", config.warnings);
    assert_eq!(fixture.tests(&config, &CANDIDATES), ["src/a.test.ts"]);
    assert_eq!(fixture.setup(&config), [PathBuf::from("src/setup.ts")]);
}

#[test]
fn applies_vitest_defaults_without_config() {
    let fixture = Fixture::new(&[]);

    let config = fixture.load(RunnerKind::Vitest);

    assert_eq!(
        fixture.tests(&config, &CANDIDATES),
        [
            "src/a.test.ts",
            "src/a.llm.test.ts",
            "src/a.spec.tsx",
            "e2e/a.spec.ts"
        ]
    );
    assert!(!config.is_test(&fixture.path("node_modules/x/a.test.ts")));
}

#[test]
fn reads_arrow_expression_and_block_bodies() {
    for source in [
        r#"export default defineConfig(() => ({ test: { include: ["e2e/**/*.spec.ts"] } }));"#,
        r#"export default defineConfig(async ({ mode }) => {
  const include = ["e2e/**/*.spec.ts"];
  return { test: { include } };
});"#,
        r#"export default defineProject({ test: { include: ["e2e/**/*.spec.ts"] } });"#,
        r#"const config = { test: { include: ["e2e/**/*.spec.ts"] } } satisfies UserConfig;
export default config;"#,
    ] {
        let fixture = Fixture::new(&[("vitest.config.ts", source)]);

        let config = fixture.load(RunnerKind::Vitest);

        assert!(
            config.warnings.is_empty(),
            "{source}: {:?}",
            config.warnings
        );
        assert_eq!(
            fixture.tests(&config, &CANDIDATES),
            ["e2e/a.spec.ts"],
            "{source}"
        );
    }
}

#[test]
fn reads_plain_esm_and_commonjs_exports() {
    for (file, source) in [
        (
            "vitest.config.mjs",
            r#"export default { test: { include: ["src/**/*.spec.tsx"] } };"#,
        ),
        (
            "vitest.config.cjs",
            r#"module.exports = { test: { include: ["src/**/*.spec.tsx"] } };"#,
        ),
    ] {
        let fixture = Fixture::new(&[(file, source)]);

        let config = fixture.load(RunnerKind::Vitest);

        assert_eq!(
            fixture.tests(&config, &CANDIDATES),
            ["src/a.spec.tsx"],
            "{file}"
        );
    }
}

#[test]
fn merge_config_follows_relative_base_and_merges_both_sides() {
    let fixture = Fixture::new(&[
        ("src/base-setup.ts", ""),
        ("src/setup.ts", ""),
        (
            "vite.config.base.ts",
            r#"export default defineConfig({ test: { include: ["src/**/*.test.ts"], setupFiles: ["./src/base-setup.ts"] } });"#,
        ),
        (
            "vitest.config.ts",
            r#"import base from "./vite.config.base";
export default mergeConfig(base, defineConfig({ test: { setupFiles: ["./src/setup.ts"] } }));"#,
        ),
    ]);

    let config = fixture.load(RunnerKind::Vitest);

    assert!(config.warnings.is_empty(), "{:?}", config.warnings);
    assert_eq!(
        fixture.tests(&config, &CANDIDATES),
        ["src/a.test.ts", "src/a.llm.test.ts"]
    );
    assert_eq!(
        fixture.setup(&config),
        [
            PathBuf::from("src/base-setup.ts"),
            PathBuf::from("src/setup.ts")
        ]
    );
}

#[test]
fn resolves_module_constants_and_spreads() {
    let fixture = Fixture::new(&[(
        "vitest.config.ts",
        r#"import { configDefaults, defineConfig } from "vitest/config";
const slow = ["**/*.llm.test.ts"] as const;
const pattern = "src/**/*." + "test.ts";
export default defineConfig({
  test: {
    include: [pattern, `e2e/**/*.spec.ts`],
    exclude: [...configDefaults.exclude, ...slow],
  },
});"#,
    )]);

    let config = fixture.load(RunnerKind::Vitest);

    assert!(config.warnings.is_empty(), "{:?}", config.warnings);
    assert_eq!(
        fixture.tests(&config, &CANDIDATES),
        ["src/a.test.ts", "e2e/a.spec.ts"]
    );
    assert!(!config.is_test(&fixture.path("dist/a.test.ts")));
}

#[test]
fn reads_vite_config_test_key_root_and_dir() {
    let fixture = Fixture::new(&[
        ("app/setup.ts", ""),
        (
            "vite.config.ts",
            r#"import path from "node:path";
export default defineConfig({
  plugins: [react()],
  root: "app",
  test: { dir: "src", setupFiles: [path.resolve(__dirname, "app/setup.ts")] },
});"#,
        ),
    ]);

    let config = fixture.load(RunnerKind::Vitest);

    assert!(config.warnings.is_empty(), "{:?}", config.warnings);
    assert!(config.is_test(&fixture.path("app/src/a.test.ts")));
    assert!(!config.is_test(&fixture.path("app/other/a.test.ts")));
    assert_eq!(fixture.setup(&config), [PathBuf::from("app/setup.ts")]);
}

#[test]
fn reads_workspace_file_with_globs_paths_and_inline_projects() {
    let fixture = Fixture::new(&[
        (
            "vitest.workspace.ts",
            r#"export default defineWorkspace([
  "packages/*",
  "!packages/skip",
  { test: { name: "e2e", include: ["e2e/**/*.spec.ts"] } },
]);"#,
        ),
        (
            "packages/app/vitest.config.ts",
            r#"export default defineProject({ test: { include: ["**/*.unit.ts"] } });"#,
        ),
        ("packages/lib/index.ts", ""),
        ("packages/skip/index.ts", ""),
    ]);

    let config = fixture.load(RunnerKind::Vitest);

    assert!(config.warnings.is_empty(), "{:?}", config.warnings);
    assert!(config.is_test(&fixture.path("packages/app/src/x.unit.ts")));
    assert!(!config.is_test(&fixture.path("packages/app/src/x.test.ts")));
    assert!(config.is_test(&fixture.path("packages/lib/x.test.ts")));
    assert!(!config.is_test(&fixture.path("packages/skip/x.test.ts")));
    assert!(config.is_test(&fixture.path("e2e/a.spec.ts")));
    assert!(config
        .project_dirs()
        .any(|dir| dir == fixture.path("packages/app")));
}

#[test]
fn reads_test_projects_with_config_paths_and_extends() {
    let fixture = Fixture::new(&[
        ("setup.ts", ""),
        (
            "vitest.config.ts",
            r#"export default defineConfig({
  test: {
    setupFiles: ["./setup.ts"],
    projects: [
      "./tools/vitest.config.ts",
      { extends: true, test: { name: "unit", include: ["src/**/*.test.ts"] } },
    ],
  },
});"#,
        ),
        (
            "tools/vitest.config.ts",
            r#"export default { test: { include: ["**/*.check.ts"] } };"#,
        ),
    ]);

    let config = fixture.load(RunnerKind::Vitest);

    assert!(config.warnings.is_empty(), "{:?}", config.warnings);
    assert!(config.is_test(&fixture.path("tools/a.check.ts")));
    assert!(config.is_test(&fixture.path("src/a.test.ts")));
    assert!(!config.is_test(&fixture.path("e2e/a.spec.ts")));
    assert!(fixture.setup(&config).contains(&PathBuf::from("setup.ts")));
}

#[test]
fn unreadable_keys_warn_and_fall_back() {
    let fixture = Fixture::new(&[(
        "vitest.config.ts",
        r#"import { computeSetup } from "./helpers";
export default defineConfig({
  test: {
    include: process.env.CI ? ["a"] : ["b"],
    setupFiles: computeSetup(),
  },
});"#,
    )]);

    let config = fixture.load(RunnerKind::Vitest);

    let keys: Vec<_> = config
        .warnings
        .iter()
        .map(|warning| warning.key.as_str())
        .collect();
    assert_eq!(keys, ["setupFiles", "include"]);
    assert!(config.warnings[0].reason.contains("computeSetup"));
    assert_eq!(config.warnings[1].fallback, HEURISTIC_FALLBACK);
    assert!(config.warnings[0].file.ends_with("vitest.config.ts"));
    assert_eq!(
        fixture.tests(&config, &CANDIDATES),
        [
            "src/a.test.ts",
            "src/a.llm.test.ts",
            "src/__tests__/kit.testkit.ts",
            "src/a.spec.tsx",
            "e2e/a.spec.ts"
        ]
    );
}
