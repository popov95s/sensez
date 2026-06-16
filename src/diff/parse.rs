//! Unified-diff (`git diff --unified=0`) hunk parser.
//!
//! Extracts, per file, the **new-side** added/modified line ranges from `@@`
//! hunk headers. Deletion-only hunks (new count 0) contribute no range — a
//! deletion has no line in the working tree to attach a finding to.

/// Parse unified-diff text into `(relative_path, [(lo, hi), ...])` entries.
pub fn parse_unified(text: &str) -> Vec<(String, Vec<(usize, usize)>)> {
    let mut out: Vec<(String, Vec<(usize, usize)>)> = Vec::new();
    let mut current: Option<usize> = None; // index into `out` for the active file

    for line in text.lines() {
        if let Some(path) = line.strip_prefix("+++ ") {
            current = new_file_path(path).map(|p| {
                out.push((p, Vec::new()));
                out.len() - 1
            });
        } else if let Some(rest) = line.strip_prefix("@@") {
            if let (Some(idx), Some(range)) = (current, new_hunk_range(rest)) {
                out[idx].1.push(range);
            }
        }
    }
    out.retain(|(_, ranges)| !ranges.is_empty());
    out
}

/// `+++ b/path` → `path`; `+++ /dev/null` (deletion) → None.
fn new_file_path(field: &str) -> Option<String> {
    let path = field.split('\t').next().unwrap_or(field).trim();
    if path == "/dev/null" {
        return None;
    }
    Some(path.strip_prefix("b/").unwrap_or(path).to_string())
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
        assert_eq!(parsed[0].0, "pkg/mod.py");
        assert_eq!(parsed[0].1, vec![(11, 13), (23, 23)]);
    }

    #[test]
    fn ignores_pure_deletions_and_dev_null() {
        let diff = "\
--- a/gone.py
+++ /dev/null
@@ -1,5 +0,0 @@
--- a/keep.py
+++ b/keep.py
@@ -5,2 +5,0 @@
";
        // /dev/null file dropped; keep.py's deletion-only hunk yields no range.
        assert!(parse_unified(diff).is_empty());
    }
}
