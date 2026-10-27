//! Files that run as programs: a shebang line or a top-level main guard.

use std::path::Path;

const MAIN_GUARDS: [&str; 5] = [
    "import.meta.main",
    "require.main ===",
    "require.main ==",
    "import.meta.url ===",
    "process.argv[1]",
];

/// Whether `path` starts with a shebang or has an unindented line holding a main guard.
pub fn is_program(path: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(path) else {
        return false;
    };
    text.starts_with("#!") || text.lines().any(is_main_guard)
}

fn is_main_guard(line: &str) -> bool {
    line.starts_with(|c: char| !c.is_whitespace() && c != '/' && c != '*')
        && MAIN_GUARDS.iter().any(|guard| line.contains(guard))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn program(source: &str) -> bool {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("tool.mjs");
        std::fs::write(&path, source).unwrap();
        is_program(&path)
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
    fn argv_read_inside_a_function_is_not_a_program() {
        assert!(!program(
            "export function readFlag() {\n  return process.argv[1];\n}\n"
        ));
        assert!(!program(
            "// if (require.main === module) run();\nexport const x = 1;\n"
        ));
    }
}
