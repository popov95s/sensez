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
