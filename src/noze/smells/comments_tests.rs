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
        narrating_min_function_lines: 0,
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
    assert_eq!(found[0].metric, 75);
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
fn rationale_directives_and_markup_do_not_count() {
    let cases = [
        (
            "ts",
            "export function ncRationale(items: number[]) {\n  // The upstream API rejects zero values, so we keep them\n  // as-is here and let the caller decide. Changing this\n  // broke the nightly import once (see incident notes).\n  // Do not normalise here.\n  // Keep this in sync with the server.\n  const kept = items.slice();\n  const first = kept[0];\n  const last = kept[kept.length - 1];\n  const size = kept.length;\n  return { kept, first, last, size };\n}\n",
        ),
        (
            "ts",
            "export function ncDirectives5(a: Legacy) {\n  // eslint-disable-next-line no-console\n  console.log(a);\n  // @ts-expect-error legacy shape\n  const b = a.x;\n  // prettier-ignore\n  const c = [1,2,3];\n  // eslint-disable-next-line no-unused-vars\n  const d = b;\n  // biome-ignore lint: legacy\n  const e = d;\n  return [b, c, e];\n}\n",
        ),
        (
            "tsx",
            "export function Page() {\n  return (\n    <div>\n      {/* header */}\n      <h1>t</h1>\n      {/* body */}\n      <p>b</p>\n      {/* list */}\n      <ul />\n      {/* footer */}\n      <footer />\n      {/* end */}\n    </div>\n  );\n}\n",
        ),
        (
            "ts",
            "export function wrap(items: number[]) {\n  return items.map((v) => {\n    // parse the value\n    // check the value\n    // keep the value\n    // map the value\n    // return the value\n    return v;\n  });\n}\n",
        ),
        (
            "ts",
            "it(\"keeps rows\", () => {\n  // load the rows\n  // filter the rows\n  // count the rows\n  // check the count\n  // done\n  const rows = load();\n  const kept = rows.filter(Boolean);\n  const count = kept.length;\n  const total = rows.length;\n  const ratio = count / total;\n  expect(ratio).toBe(1);\n});\n",
        ),
    ];
    for (ext, body) in cases {
        let file = parsed("quiet", ext, body);
        let found = findings(&file, &Smells::default());
        assert!(found.is_empty(), "{body}\n{found:?}");
    }
}

#[test]
fn step_narration_and_commented_out_code_keep_firing() {
    let narration = parsed(
        "narration",
        "ts",
        "export function parse(raw: string): number {\n  // parse the integer\n  const n = parseInt(raw, 10);\n  // check if negative\n  if (n < 0) {\n    // return zero\n    return 0;\n  }\n  // double it\n  const d = n * 2;\n  // return the result\n  return d;\n}\n",
    );
    let found = findings(&narration, &Smells::default());
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].line, 2);
    assert!(
        found[0].message.contains("narrating"),
        "{}",
        found[0].message
    );

    let code = parsed(
        "code",
        "ts",
        "export function old(x: number): number {\n  // const y = x * 3;\n  // if (y > 10) {\n  //   return y;\n  // }\n  // console.log(y);\n  return x;\n}\n",
    );
    let found = findings(&code, &Smells::default());
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].line, 2);
    assert!(
        found[0].message.contains("5 line(s) of commented-out code"),
        "{}",
        found[0].message
    );
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
