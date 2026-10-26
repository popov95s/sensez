use super::*;

#[test]
fn identifiers_split_on_case_underscores_and_acronyms() {
    assert_eq!(split_identifier("parseCsvRow"), ["parse", "csv", "row"]);
    assert_eq!(split_identifier("parse_csv_row"), ["parse", "csv", "row"]);
    assert_eq!(split_identifier("HTTPServer"), ["http", "server"]);
    assert_eq!(split_identifier("utf8Decoder"), ["utf8", "decoder"]);
}

#[test]
fn signature_words_skip_filler_and_repeat_nothing() {
    let func = crate::spine::parser::FunctionUnit {
        name: "getActiveUserEmails".into(),
        param_names: vec!["users".into(), "activeOnly".into()],
        ..Default::default()
    };
    assert_eq!(
        signature_words(&func),
        ["active", "user", "emails", "users", "only"]
    );
}
