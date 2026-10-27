//! Files that run as programs: a shebang line or a top-level main guard.

const MAIN_GUARDS: [&str; 5] = [
    "import.meta.main",
    "require.main ===",
    "require.main ==",
    "import.meta.url ===",
    "process.argv[1]",
];

/// Whether `src` starts with a shebang or has an unindented line holding a main guard.
pub fn is_program(src: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(src) else {
        return false;
    };
    text.starts_with("#!") || has_main_guard(text)
}

fn has_main_guard(text: &str) -> bool {
    let mut lines = text.lines().peekable();
    while let Some(line) = lines.next() {
        if !starts_statement(line) {
            continue;
        }
        let continuation = lines
            .next_if(|next| !line.ends_with(['{', ';']) && next.starts_with(char::is_whitespace))
            .unwrap_or("");
        if MAIN_GUARDS
            .iter()
            .any(|guard| line.contains(guard) || continuation.contains(guard))
        {
            return true;
        }
    }
    false
}

fn starts_statement(line: &str) -> bool {
    line.starts_with(|c: char| !c.is_whitespace() && c != '/' && c != '*')
}

#[cfg(test)]
mod tests {
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
}
