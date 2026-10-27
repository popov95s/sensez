use super::dependency_reimplementation::detect;
use super::test_support::Project;
use crate::config::smells::{SmellConfig, Smells};
use crate::report::{ActionLevel, SmellFinding};

const TOOLKIT: &str = r#"{"dependencies": {"es-toolkit": "^1.0.0"}}"#;
const UNIQUE: &str =
    "export function unique<T>(items: T[]): T[] {\n  return Array.from(new Set(items));\n}\n";

fn findings(files: &[(&str, &str)]) -> Vec<SmellFinding> {
    findings_with(files, &SmellConfig::default())
}

fn findings_with(files: &[(&str, &str)], cfg: &SmellConfig) -> Vec<SmellFinding> {
    detect(&Project::new(files).parsed(), cfg)
}

#[test]
fn copy_of_a_listed_package_utility_names_the_provider() {
    let found = findings(&[("package.json", TOOLKIT), ("src/list.ts", UNIQUE)]);

    assert_eq!(found.len(), 1);
    assert_eq!(
        found[0].message,
        "`unique` re-implements `uniq` from es-toolkit, which package.json already lists"
    );
    assert_eq!(found[0].action, ActionLevel::Info);
}

#[test]
fn matching_name_with_another_arity_is_quiet() {
    let source = "export function unique(items: string[], key: string) {\n  return items.filter((item) => item !== key);\n}\n";

    assert!(findings(&[("package.json", TOOLKIT), ("src/list.ts", source)]).is_empty());
}

#[test]
fn matching_name_with_a_long_body_is_quiet() {
    let body = "  const seen = new Set();\n".repeat(8);
    let source = format!("export function unique(items: string[]) {{\n{body}  return items;\n}}\n");

    assert!(findings(&[("package.json", TOOLKIT), ("src/list.ts", &source)]).is_empty());
}

#[test]
fn wrapper_in_a_file_importing_the_provider_is_quiet() {
    let source = format!("import {{ uniq }} from \"es-toolkit\";\n{UNIQUE}");

    assert!(findings(&[("package.json", TOOLKIT), ("src/list.ts", &source)]).is_empty());
}

#[test]
fn dev_only_provider_does_not_count_for_production_code() {
    let manifest = r#"{"devDependencies": {"es-toolkit": "^1.0.0"}}"#;

    assert!(findings(&[("package.json", manifest), ("src/list.ts", UNIQUE)]).is_empty());
}

#[test]
fn utility_without_a_declared_provider_is_quiet() {
    let source = "export function sleep(ms: number) {\n  return new Promise((done) => setTimeout(done, ms));\n}\n";

    assert!(findings(&[("package.json", "{}"), ("src/time.ts", source)]).is_empty());
}

#[test]
fn methods_named_like_utilities_are_quiet() {
    let source = "export class Bag {\n  unique(items: string[]) {\n    return [...new Set(items)];\n  }\n}\n";

    assert!(findings(&[("package.json", TOOLKIT), ("src/bag.ts", source)]).is_empty());
}

#[test]
fn python_standard_library_counts_without_a_manifest() {
    let source = "def chunks(items, size):\n    return [items[i:i + size] for i in range(0, len(items), size)]\n";

    let found = findings(&[("app/batching.py", source)]);

    assert_eq!(found.len(), 1);
    assert_eq!(
        found[0].message,
        "`chunks` re-implements `itertools.batched` from the standard library"
    );
}

#[test]
fn python_manifest_package_is_suggested() {
    let pyproject = "[project]\ndependencies = [\"tenacity>=8\"]\n";
    let source = "def retry(action):\n    for _ in range(3):\n        try:\n            return action()\n        except OSError:\n            pass\n";

    let found = findings(&[("pyproject.toml", pyproject), ("app/net.py", source)]);

    assert_eq!(
        found[0].message,
        "`retry` re-implements `retry` from tenacity, which pyproject.toml already lists"
    );
}

#[test]
fn configured_in_house_package_counts_as_a_provider() {
    let source = "export function clamp(value: number, low: number, high: number) {\n  return Math.min(Math.max(value, low), high);\n}\n";
    let cfg = SmellConfig::from(Smells {
        dependency_reimplementation_providers: vec!["@acme/kit".to_string()],
        ..Smells::default()
    });

    let found = findings_with(&[("package.json", "{}"), ("src/math.ts", source)], &cfg);

    assert_eq!(
        found[0].message,
        "`clamp` re-implements `clamp` from @acme/kit, listed in dependency_reimplementation_providers"
    );
}

#[test]
fn three_copies_across_the_project_are_advisory() {
    let found = findings(&[
        ("package.json", TOOLKIT),
        ("src/a.ts", UNIQUE),
        ("src/b.ts", UNIQUE),
        ("src/c.ts", UNIQUE),
    ]);

    assert_eq!(found.len(), 3);
    assert!(found
        .iter()
        .all(|finding| finding.action == ActionLevel::Advisory));
    assert!(found[0].message.ends_with("; 3 copies across the project"));
}

#[test]
fn helpers_nested_in_another_function_are_quiet() {
    let source = "export const run = async () => {\n  const sleep = (ms: number) => new Promise((done) => setTimeout(done, ms));\n  await sleep(5);\n};\n";

    assert!(findings(&[("package.json", TOOLKIT), ("src/run.ts", source)]).is_empty());
}
