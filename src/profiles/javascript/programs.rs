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
#[path = "programs_tests.rs"]
mod tests;
