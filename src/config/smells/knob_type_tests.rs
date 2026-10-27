//! Wrong-typed rule knobs name the type they expect.

use super::SmellConfig;

fn error(toml: &str) -> String {
    toml::from_str::<SmellConfig>(toml).unwrap_err().to_string()
}

#[test]
fn bool_knob_given_a_string_asks_for_a_boolean() {
    let err = error("[rules.literal_membership]\nskip_external_vocab = \"yes\"\n");
    assert!(
        err.contains("skip_external_vocab must be a boolean"),
        "{err}"
    );
}

#[test]
fn list_knob_given_a_string_asks_for_a_list() {
    let err = error("[rules.message_chain]\nignore_roots = \"api\"\n");
    assert!(
        err.contains("ignore_roots must be a list of strings"),
        "{err}"
    );
}

#[test]
fn integer_knob_given_a_bool_asks_for_an_integer() {
    let err = error("[rules.long_function]\nmax_lines = true\n");
    assert!(err.contains("max_lines must be an integer"), "{err}");
}

#[test]
fn negative_integer_knob_asks_for_non_negative() {
    let err = error("[rules.long_function]\nmax_lines = -1\n");
    assert!(err.contains("max_lines must be non-negative"), "{err}");
}
