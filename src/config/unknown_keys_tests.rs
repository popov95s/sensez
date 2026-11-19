use super::*;

fn keys(text: &str) -> Vec<(String, KeyPolicy)> {
    let doc = toml_edit::ImDocument::parse(text).unwrap();
    let mut unknown: Vec<_> = collect(doc.as_table())
        .into_iter()
        .map(|key| (key.path, key.policy))
        .collect();
    unknown.sort_by(|a, b| a.0.cmp(&b.0));
    unknown
}

fn paths(text: &str) -> Vec<String> {
    keys(text).into_iter().map(|(path, _)| path).collect()
}

#[test]
fn valid_config_has_no_unknown_keys() {
    assert!(paths("[duplication]\nthreshold = 50\n[dead_code]\nunused_methods = true\n[gate]\nrepeat_limit = 3\n").is_empty());
}

#[test]
fn typos_are_qualified_by_section() {
    let unknown = paths("treshold = 5\n[duplication]\ntreshold = 40\n");
    assert_eq!(unknown, vec!["duplication.treshold", "treshold"]);
}

#[test]
fn semantic_and_forbidden_entries_are_checked() {
    let unknown = paths(
        "[duplication.semantic]\nmin_shape = 80\n[[boundaries.forbidden]]\nfrm = \"a\"\nto = \"b\"\n",
    );
    assert_eq!(
        unknown,
        vec![
            "boundaries.forbidden[0].frm",
            "duplication.semantic.min_shape"
        ]
    );
}

#[test]
fn free_form_sections_are_not_flagged() {
    let unknown =
        paths("[accept]\nanything_at_all = [\"x\"]\n[action.smells]\nwhatever = \"info\"\n");
    assert!(unknown.is_empty());
}

#[test]
fn smells_and_action_keys_are_rejected_others_warn() {
    let unknown = keys(
        "[deadcode]\nx = 1\n[action]\ncycle = \"info\"\n[smells]\ntypo = 1\n\
         [smells.python]\nmade_up = 1\n[smells.rules.long_function]\nmax_line = 20\n\
         [smells.rust.rules.large_class]\nmax_method = 3\n",
    );
    assert_eq!(
        unknown,
        vec![
            ("action.cycle".to_string(), KeyPolicy::Reject),
            ("deadcode".to_string(), KeyPolicy::Warn),
            ("smells.python.made_up".to_string(), KeyPolicy::Reject),
            (
                "smells.rules.long_function.max_line".to_string(),
                KeyPolicy::Reject
            ),
            (
                "smells.rust.rules.large_class.max_method".to_string(),
                KeyPolicy::Reject
            ),
            ("smells.typo".to_string(), KeyPolicy::Reject),
        ]
    );
}

#[test]
fn unknown_keys_carry_spans_and_suggestions() {
    let doc =
        toml_edit::ImDocument::parse("[smells.rules.long_function]\nmax_line = 20\n").unwrap();
    let unknown = collect(doc.as_table());
    assert_eq!(unknown.len(), 1);
    assert_eq!(unknown[0].span, Some(29..37));
    assert_eq!(unknown[0].suggestion.as_deref(), Some("max_lines"));
}

#[test]
fn renamed_dead_code_keys_accept_their_old_names() {
    let renamed = "[dead_code]\nregistration_decorators = []\nentry_globs = []\n";
    let old = "[dead_code]\nentrypoints = []\nentry_points = []\n";
    assert!(paths(renamed).is_empty());
    assert!(paths(old).is_empty());
}
