use super::tests::Project;

fn deeply_nested_type() -> String {
    format!(
        "export type Deep = {}number{};\n",
        "Array<".repeat(600),
        ">".repeat(600)
    )
}

#[test]
fn unparsable_declaration_file_keeps_the_plan_narrow() {
    let deep = deeply_nested_type();
    let project = Project::with_files(&[
        ("src/feature.ts", "export const value = 1;\n"),
        ("src/generated.d.ts", &deep),
        (
            "src/feature.test.ts",
            "import { value } from './feature'; test('f', () => value);\n",
        ),
        ("src/other.test.ts", "test('o', () => 1);\n"),
    ]);
    project.change("src/feature.ts", "export const value = 2;\n");

    let plan = project.plan();

    assert!(!plan.full_suite, "{:?}", plan.fallback_reasons);
    assert_eq!(project.selected(&plan), ["src/feature.test.ts"]);
}

#[test]
fn unparsable_source_file_with_imports_falls_back_to_every_test() {
    let deep = format!("import './feature';\n{}", deeply_nested_type());
    let project = Project::with_files(&[
        ("src/feature.ts", "export const value = 1;\n"),
        ("src/generated.ts", &deep),
        (
            "src/feature.test.ts",
            "import { value } from './feature'; test('f', () => value);\n",
        ),
        ("src/other.test.ts", "test('o', () => 1);\n"),
    ]);
    project.change("src/feature.ts", "export const value = 2;\n");

    let plan = project.plan();

    assert!(plan.full_suite);
}

fn long_flat_chain() -> String {
    let terms: Vec<_> = (0..700).map(|index| format!("x{index}")).collect();
    format!("export const total = {};\n", terms.join(" + "))
}

#[test]
fn unparsable_file_without_imports_keeps_the_plan_narrow() {
    let chain = long_flat_chain();
    let project = Project::with_files(&[
        ("src/feature.ts", "export const value = 1;\n"),
        ("src/chain.ts", &chain),
        (
            "src/feature.test.ts",
            "import { value } from './feature'; test('f', () => value);\n",
        ),
        ("src/other.test.ts", "test('o', () => 1);\n"),
    ]);
    project.change("src/feature.ts", "export const value = 2;\n");

    let plan = project.plan();

    assert!(!plan.full_suite, "{:?}", plan.fallback_reasons);
    assert_eq!(project.selected(&plan), ["src/feature.test.ts"]);
}

#[test]
fn changed_unparsable_file_without_imports_falls_back() {
    let chain = long_flat_chain();
    let project = Project::with_files(&[
        ("src/chain.ts", &chain),
        ("src/other.test.ts", "test('o', () => 1);\n"),
    ]);
    project.change("src/chain.ts", &format!("{chain}// edited\n"));

    let plan = project.plan();

    assert!(plan.full_suite);
}
