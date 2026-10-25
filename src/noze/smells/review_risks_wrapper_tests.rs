use super::*;
use crate::noze::smells::test_support as fixture;

#[test]
fn unchanged_local_wrapper_chain_is_reported() {
    let mut config = Smells::default();
    config
        .disabled
        .retain(|kind| *kind != SmellKind::RedundantWrapperChain);
    for (ext, source) in [
        ("py", "def outer(value):\n    return middle(value)\ndef middle(value):\n    return actual(value)\ndef actual(value):\n    return value * 2\n"),
        ("js", "function outer(value) { return middle(value); }\nfunction middle(value) { return actual(value); }\nfunction actual(value) { return value * 2; }\n"),
    ] {
        let file = fixture::parsed(&format!("sample.{ext}"), source);
        let findings = detect_local(&file, &config);
        assert_eq!(findings.iter().filter(|f| f.kind == SmellKind::RedundantWrapperChain).count(), 1, "{ext}");
    }
}

#[test]
fn wrapper_chain_depth_defaults_to_zero_and_can_be_overridden() {
    let one_wrapper =
        "def outer(value):\n    return actual(value)\ndef actual(value):\n    return value * 2\n";
    let mut config = Smells::default();
    assert_eq!(config.max_wrapper_depth, 0);
    assert!(has(
        &local_with_config("py", one_wrapper, &config),
        SmellKind::RedundantWrapperChain
    ));

    config.max_wrapper_depth = 1;
    assert!(!has(
        &local_with_config("py", one_wrapper, &config),
        SmellKind::RedundantWrapperChain
    ));

    let two_wrappers = "def outer(value):\n    return middle(value)\ndef middle(value):\n    return actual(value)\ndef actual(value):\n    return value * 2\n";
    assert!(has(
        &local_with_config("py", two_wrappers, &config),
        SmellKind::RedundantWrapperChain
    ));

    config.max_wrapper_depth = 2;
    assert!(!has(
        &local_with_config("py", two_wrappers, &config),
        SmellKind::RedundantWrapperChain
    ));
}

#[test]
fn argument_changes_are_not_reported_as_wrappers() {
    let source = "def outer(value):\n    return actual(value.strip())\ndef actual(value):\n    return value\n";
    assert!(!has(&local("py", source), SmellKind::RedundantWrapperChain));
    let source = "function outer(value) { return actual(value.trim()); }\nfunction actual(value) { return value; }\n";
    assert!(!has(&local("js", source), SmellKind::RedundantWrapperChain));
}
