use super::detect_local;
use crate::config::smells::Smells;
use crate::report::SmellKind;
use crate::spine::parser::{parse_file, ParsedFile};
use std::fs;

fn parsed(name: &str, ext: &str, body: &str) -> ParsedFile {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join(format!("{name}.{ext}"));
    fs::write(&path, body).unwrap();
    parse_file(&path, 0).unwrap()
}

fn findings(file: &ParsedFile, cfg: &Smells) -> Vec<crate::report::SmellFinding> {
    detect_local(file, cfg)
        .into_iter()
        .filter(|finding| finding.kind == SmellKind::NarratingCode)
        .collect()
}

fn cfg() -> Smells {
    Smells {
        min_comment_lines: 3,
        max_comment_ratio_percent: 30,
        ..Smells::default()
    }
}

#[test]
fn python_flags_dense_explanatory_comments() {
    let file = parsed(
        "dense",
        "py",
        "def ship(order):\n    # get user\n    user = order.user\n    # get plan\n    plan = user.plan\n    # check plan\n    return plan.active\n",
    );
    let found = findings(&file, &cfg());
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].metric, 43);
}

#[test]
fn minimum_comment_lines_prevents_single_helpful_note_noise() {
    let file = parsed(
        "why",
        "py",
        "def retry(job):\n    # The upstream API applies the new quota at midnight UTC.\n    return job.next_window()\n",
    );
    assert!(findings(&file, &cfg()).is_empty());
}

#[test]
fn high_comment_count_below_ratio_is_allowed() {
    let file = parsed(
        "spread_out",
        "py",
        &format!(
            "def transform(items):\n{}{}    return items\n",
            "    # phase marker\n".repeat(3),
            "    value = 1\n".repeat(20)
        ),
    );
    assert!(findings(&file, &cfg()).is_empty());
}

#[test]
fn typescript_counts_multiline_block_comments() {
    let file = parsed(
        "block",
        "ts",
        "function ship(order: Order) {\n  /* read customer\n     read plan\n     read status */\n  return order.customer.plan.active;\n}\n",
    );
    assert_eq!(findings(&file, &cfg()).len(), 1);
}

#[test]
fn rust_counts_line_comments_in_functions() {
    let file = parsed(
        "narrate",
        "rs",
        "pub fn ship(order: &Order) -> bool {\n    // read customer\n    let customer = order.customer();\n    // read plan\n    let plan = customer.plan();\n    // read active\n    plan.active()\n}\n",
    );
    assert_eq!(findings(&file, &cfg()).len(), 1);
}

#[test]
fn rule_can_disable_narrating_code() {
    let mut off = cfg();
    off.narrating_code = false;
    let file = parsed(
        "off",
        "py",
        "def ship(order):\n    # get user\n    user = order.user\n    # get plan\n    plan = user.plan\n    # check plan\n    return plan.active\n",
    );
    assert!(findings(&file, &off).is_empty());
}
