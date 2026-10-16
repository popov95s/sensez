/// Rewrite a picomatch glob into globset syntax (`?(a|b)` → `{,a,b}`, `@(a|b)`/`+(a|b)` → `{a,b}`).
pub fn to_globset(pattern: &str) -> Result<String, String> {
    let mut rest = pattern.strip_prefix("./").unwrap_or(pattern);
    let mut out = String::new();
    while let Some(start) = extglob_start(rest) {
        let (before, group) = rest.split_at(start);
        out.push_str(before);
        if is_inside_braces(&out) {
            return Err(format!("nested extglob in `{pattern}` is not supported"));
        }
        let close =
            closing_paren(group).ok_or_else(|| format!("unbalanced extglob in `{pattern}`"))?;
        let kind = group.chars().next().unwrap_or_default();
        out.push_str(&alternation(kind, &group[2..close], pattern)?);
        rest = &group[close + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

fn extglob_start(text: &str) -> Option<usize> {
    text.char_indices()
        .find(|(index, current)| {
            matches!(current, '?' | '@' | '+' | '*' | '!') && text[index + 1..].starts_with('(')
        })
        .map(|(index, _)| index)
}

fn alternation(kind: char, inner: &str, pattern: &str) -> Result<String, String> {
    let options: Vec<&str> = inner.split('|').collect();
    if options
        .iter()
        .any(|option| option.contains(['{', ',', '(', '|']))
    {
        return Err(format!("complex extglob in `{pattern}` is not supported"));
    }
    match kind {
        '?' => Ok(format!("{{,{}}}", options.join(","))),
        // `+(a|b)` is approximated as exactly one repetition.
        '@' | '+' => Ok(format!("{{{}}}", options.join(","))),
        _ => Err(format!(
            "extglob `{kind}(...)` in `{pattern}` is not supported"
        )),
    }
}

fn closing_paren(group: &str) -> Option<usize> {
    group
        .char_indices()
        .scan(0isize, |depth, (index, current)| {
            *depth += nesting_step(current, ('(', ')'));
            Some((index, current, *depth))
        })
        .find(|(_, current, depth)| *current == ')' && *depth == 0)
        .map(|(index, _, _)| index)
}

fn is_inside_braces(text: &str) -> bool {
    text.chars()
        .map(|current| nesting_step(current, ('{', '}')))
        .sum::<isize>()
        > 0
}

fn nesting_step(current: char, (open, close): (char, char)) -> isize {
    match current {
        _ if current == open => 1,
        _ if current == close => -1,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::super::defaults::{JEST_TEST_MATCH, VITEST_EXCLUDE, VITEST_INCLUDE};
    use super::*;

    fn matcher(patterns: &[&str]) -> globset::GlobSet {
        let owned: Vec<String> = patterns.iter().map(|pattern| pattern.to_string()).collect();
        super::super::glob_set(&owned).unwrap()
    }

    #[test]
    fn translates_optional_and_one_of_groups() {
        assert_eq!(
            to_globset("**/*.{test,spec}.?(c|m)[jt]s?(x)").unwrap(),
            "**/*.{test,spec}.{,c,m}[jt]s{,x}"
        );
        assert_eq!(to_globset("./src/@(a|b).ts").unwrap(), "src/{a,b}.ts");
        assert_eq!(
            to_globset("**/?(*.)+(spec|test).?([mc])[jt]s?(x)").unwrap(),
            "**/{,*.}{spec,test}.{,[mc]}[jt]s{,x}"
        );
    }

    #[test]
    fn rejects_unsupported_extglobs() {
        assert!(to_globset("**/!(skip).ts").is_err());
        assert!(to_globset("**/*(a|b).ts").is_err());
        assert!(to_globset("{a,?(b|c)}.ts").is_err());
    }

    #[test]
    fn vitest_default_include_matches_runner_semantics() {
        let include = matcher(&VITEST_INCLUDE);
        for path in [
            "a.test.ts",
            "src/a.spec.tsx",
            "src/a.test.mjs",
            "src/a.test.cjs",
            "src/a.spec.jsx",
        ] {
            assert!(include.is_match(path), "{path}");
        }
        for path in [
            "src/__tests__/helper.ts",
            "src/a.test.css",
            "src/a.testkit.ts",
        ] {
            assert!(!include.is_match(path), "{path}");
        }
    }

    #[test]
    fn vitest_default_exclude_matches_runner_semantics() {
        let exclude = matcher(&VITEST_EXCLUDE);
        for path in [
            "node_modules/x/a.test.ts",
            "packages/app/dist/a.test.js",
            ".git/a.test.ts",
            "vitest.config.ts",
        ] {
            assert!(exclude.is_match(path), "{path}");
        }
        assert!(!exclude.is_match("src/a.test.ts"));
    }

    #[test]
    fn jest_default_test_match_keeps_tests_dirs() {
        let include = matcher(&JEST_TEST_MATCH);
        for path in [
            "/repo/src/__tests__/thing.js",
            "/repo/src/a.test.ts",
            "/repo/src/a.spec.mjs",
            "/repo/test.js",
        ] {
            assert!(include.is_match(path), "{path}");
        }
        assert!(!include.is_match("/repo/src/helper.ts"));
    }
}
