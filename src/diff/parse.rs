//! Unified-diff (`git diff --unified=0`) hunk parser.
//!
//! Extracts, per file, the **new-side** added/modified line ranges from `@@`
//! hunk headers, and the text of every removed line.

/// One file section of a unified diff.
#[derive(Debug, Default, PartialEq)]
pub struct FileDiff {
    /// New-side path; `None` when the file was deleted.
    pub path: Option<String>,
    pub ranges: Vec<(usize, usize)>,
    pub removed: Vec<String>,
}

/// Parse unified-diff text into one [`FileDiff`] per file section.
pub fn parse_unified(text: &str) -> Vec<FileDiff> {
    let mut sections: Vec<Vec<&str>> = Vec::new();
    for line in text.lines() {
        if line.starts_with("diff --git ") {
            sections.push(Vec::new());
        } else if let Some(section) = sections.last_mut() {
            section.push(line);
        }
    }
    sections.into_iter().filter_map(parse_section).collect()
}

fn parse_section(section: Vec<&str>) -> Option<FileDiff> {
    let mut lines = section
        .into_iter()
        .skip_while(|line| !line.starts_with("+++ "));
    let mut file = FileDiff {
        path: new_file_path(lines.next()?.strip_prefix("+++ ")?.trim_start()),
        ..FileDiff::default()
    };
    for line in lines {
        if let Some(hunk) = line.strip_prefix("@@") {
            file.ranges.extend(new_hunk_range(hunk));
        } else if let Some(removed) = line.strip_prefix('-') {
            file.removed.push(removed.to_string());
        }
    }
    Some(file)
}

/// `+++ b/path` or `+++ "b/quoted"` → `path`; `+++ /dev/null` (deletion) → None.
fn new_file_path(field: &str) -> Option<String> {
    let path = match field.strip_prefix('"') {
        Some(quoted) => unquote(quoted),
        None => field.split('\t').next().unwrap_or(field).trim().to_string(),
    };
    if path == "/dev/null" {
        return None;
    }
    Some(path.strip_prefix("b/").unwrap_or(&path).to_string())
}

/// Decode git's C-style quoted path body (after the opening quote) up to the closing quote.
fn unquote(quoted: &str) -> String {
    let mut bytes = Vec::new();
    let mut chars = quoted.bytes().peekable();
    while let Some(byte) = chars.next() {
        match byte {
            b'"' => break,
            b'\\' => match chars.next() {
                Some(digit @ b'0'..=b'7') => {
                    let mut value = u32::from(digit - b'0');
                    for _ in 0..2 {
                        if let Some(next @ b'0'..=b'7') = chars.peek().copied() {
                            value = value * 8 + u32::from(next - b'0');
                            chars.next();
                        }
                    }
                    bytes.push(u8::try_from(value).unwrap_or(b'?'));
                }
                Some(b'n') => bytes.push(b'\n'),
                Some(b't') => bytes.push(b'\t'),
                Some(b'r') => bytes.push(b'\r'),
                Some(b'a') => bytes.push(0x07),
                Some(b'b') => bytes.push(0x08),
                Some(b'f') => bytes.push(0x0c),
                Some(b'v') => bytes.push(0x0b),
                Some(other) => bytes.push(other),
                None => {}
            },
            other => bytes.push(other),
        }
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

/// From `@@ -l,s +l,s @@`, return the new-side `[l, l+s-1]` (s defaults to 1).
fn new_hunk_range(rest: &str) -> Option<(usize, usize)> {
    let plus = rest.split('+').nth(1)?;
    let spec = plus.split([' ', '@']).next()?;
    let mut parts = spec.split(',');
    let start: usize = parts.next()?.trim().parse().ok()?;
    let count: usize = parts.next().map_or(Some(1), |c| c.trim().parse().ok())?;
    if count == 0 {
        return None; // pure deletion — no new-side lines
    }
    Some((start, start + count - 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn added_content_starting_with_plus_signs_stays_content() {
        let diff = concat!(
            "diff --git a/docs/guide.md b/docs/guide.md\n",
            "--- a/docs/guide.md\n",
            "+++ b/docs/guide.md\n",
            "@@ -1,0 +2,1 @@\n",
            "+++ this added line looks like a header\n",
        );
        let parsed = parse_unified(diff);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].path.as_deref(), Some("docs/guide.md"));
        assert_eq!(parsed[0].ranges, vec![(2, 2)]);
    }

    #[test]
    fn parses_added_and_modified_hunks() {
        let diff = "\
diff --git a/pkg/mod.py b/pkg/mod.py
index 111..222 100644
--- a/pkg/mod.py
+++ b/pkg/mod.py
@@ -10,0 +11,3 @@
@@ -20,1 +23,1 @@
";
        let parsed = parse_unified(diff);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].path.as_deref(), Some("pkg/mod.py"));
        assert_eq!(parsed[0].ranges, vec![(11, 13), (23, 23)]);
    }

    #[test]
    fn pure_deletions_and_dev_null_add_no_range() {
        let diff = "\
diff --git a/gone.py b/gone.py
--- a/gone.py
+++ /dev/null
@@ -1,1 +0,0 @@
-import keep
diff --git a/keep.py b/keep.py
--- a/keep.py
+++ b/keep.py
@@ -5,1 +5,0 @@
-    return helper()
";
        let parsed = parse_unified(diff);
        assert_eq!(parsed[0].path, None);
        assert_eq!(parsed[0].removed, vec!["import keep"]);
        assert_eq!(parsed[1].path.as_deref(), Some("keep.py"));
        assert!(parsed[1].ranges.is_empty());
        assert_eq!(parsed[1].removed, vec!["    return helper()"]);
    }

    #[test]
    fn quoted_non_ascii_path_is_unescaped() {
        let diff = concat!(
            "diff --git \"a/\\303\\274mlaut.py\" \"b/\\303\\274mlaut.py\"\n",
            "--- \"a/\\303\\274mlaut.py\"\n",
            "+++ \"b/\\303\\274mlaut.py\"\n",
            "@@ -3,0 +4,2 @@\n",
        );
        let parsed = parse_unified(diff);
        assert_eq!(parsed[0].path.as_deref(), Some("ümlaut.py"));
        assert_eq!(parsed[0].ranges, vec![(4, 5)]);
    }
}
