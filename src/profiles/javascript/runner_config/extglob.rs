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
#[path = "extglob_tests.rs"]
mod tests;
