use super::*;

fn program(source: &str) -> bool {
    is_program(source.as_bytes())
}

#[test]
fn shebang_marks_a_program() {
    assert!(program("#!/usr/bin/env node\nexport const x = 1;\n"));
}

#[test]
fn top_level_main_guards_mark_a_program() {
    assert!(program("if (import.meta.main) {\n  run();\n}\n"));
    assert!(program("if (require.main === module) run();\n"));
    assert!(program(
        "if (import.meta.url === `file://${process.argv[1]}`) {\n  run();\n}\n"
    ));
    assert!(program(
        "const isMain = process.argv[1] === fileURLToPath(import.meta.url);\n"
    ));
}

#[test]
fn guard_split_over_two_lines_marks_a_program() {
    assert!(program(
        "const isEntry =\n  import.meta.url === pathToFileURL(process.argv[1]).href;\nif (isEntry) run();\n"
    ));
}

#[test]
fn argv_read_inside_a_function_is_not_a_program() {
    assert!(!program(
        "export function readFlag() {\n  return process.argv[1];\n}\n"
    ));
    assert!(!program(
        "// if (require.main === module) run();\nexport const x = 1;\n"
    ));
}
