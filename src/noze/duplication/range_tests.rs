use super::detect;
use super::test_support::write_files;
use crate::config::model::Duplication;

const HELPER: &str = "export async function withRetry(fn, attempts) {\n  let lastError = null;\n  for (let i = 0; i < attempts; i++) {\n    try {\n      return await fn();\n    } catch (err) {\n      lastError = err;\n      console.warn(\"retrying\", i, err);\n      await sleep(250 * (i + 1));\n    }\n  }\n  throw lastError;\n}\n";

const RETRY_BODY: &str = "  let lastError = null;\n  for (let i = 0; i < attempts; i++) {\n    try {\n      return await fn();\n    } catch (err) {\n      lastError = err;\n      console.warn(\"retrying\", i, err);\n      await sleep(250 * (i + 1));\n    }\n  }\n";

type Summary = Vec<(usize, Vec<(usize, usize)>)>;

fn clones(files: &[(&str, &str)], threshold: usize, max_gap: usize) -> Summary {
    let tmp = tempfile::tempdir().unwrap();
    let parsed = write_files(tmp.path(), files);
    let config = Duplication {
        exclude: vec![],
        threshold,
        max_gap,
        ..Default::default()
    };
    detect(&parsed, &config)
        .into_iter()
        .map(|c| {
            let rows = c.occurrences.iter().map(|o| (o.start_row, o.end_row));
            (c.token_length, rows.collect())
        })
        .collect()
}

fn helper_then(tail_a: &str, tail_b: &str) -> [(String, String); 2] {
    [
        ("a.ts".into(), format!("{HELPER}\n{tail_a}")),
        ("b.ts".into(), format!("{HELPER}\n{tail_b}")),
    ]
}

fn refs(files: &[(String, String)]) -> Vec<(&str, &str)> {
    files
        .iter()
        .map(|(n, b)| (n.as_str(), b.as_str()))
        .collect()
}

#[test]
fn helper_before_different_declarations_ends_at_its_brace() {
    let files = helper_then(
        "const loadUsers = async (client) => {\n  const rows = await client.query(\"users\");\n  const out = [];\n  for (const row of rows) {\n    out.push(row.id);\n  }\n  logger.info(\"loaded\", out.length);\n  return out;\n};\n",
        "const saveOrder = async (order) => {\n  return api.post(\"/orders\", order);\n};\n",
    );
    assert_eq!(
        clones(&refs(&files), 30, 0),
        vec![(37, vec![(1, 13), (1, 13)])]
    );
}

#[test]
fn straddled_header_clone_ends_on_shared_header_line() {
    let files = helper_then(
        "const main = async (input) => {\n  return api.post(\"/a\", input);\n};\n",
        "const main = async (input) => {\n  logger.info(\"b\");\n};\n",
    );
    assert_eq!(
        clones(&refs(&files), 30, 0),
        vec![(41, vec![(1, 15), (1, 15)])]
    );
}

#[test]
fn whole_file_copy_is_kept_through_its_brace() {
    assert_eq!(
        clones(&[("a.ts", HELPER), ("b.ts", HELPER)], 30, 0),
        vec![(37, vec![(1, 13), (1, 13)])]
    );
}

#[test]
fn clone_ending_in_multi_line_call_is_kept() {
    let head = "logger.info(\"start\");\nconst cfg = loadConfig(\"app\");\nawait client.connect(cfg.url);\nawait client.send(\n  \"topic\",\n  cfg.payload,\n  { retries: 3 },\n);\n";
    let a = format!("{head}process.exit(0);\n");
    let b = format!("{head}console.log(cfg.url);\n");
    assert_eq!(
        clones(&[("a.ts", &a), ("b.ts", &b)], 20, 0),
        vec![(22, vec![(1, 8), (1, 8)])]
    );
}

#[test]
fn statements_inside_function_end_at_last_statement() {
    let a = format!("function one(fn, attempts) {{\n{RETRY_BODY}  throw lastError;\n}}\n");
    let b = format!("function two(fn, attempts) {{\n{RETRY_BODY}  return null;\n}}\n");
    assert_eq!(
        clones(&[("a.ts", &a), ("b.ts", &b)], 30, 0),
        vec![(34, vec![(1, 11), (1, 11)])]
    );
}

#[test]
fn enclosing_function_with_divergent_tail_is_kept() {
    let a = format!("function same(fn, attempts) {{\n{RETRY_BODY}  throw lastError;\n}}\n");
    let b = format!("function same(fn, attempts) {{\n{RETRY_BODY}  return null;\n}}\n");
    assert_eq!(
        clones(&[("a.ts", &a), ("b.ts", &b)], 30, 0),
        vec![(36, vec![(1, 11), (1, 11)])]
    );
}

#[test]
fn stitched_cluster_ends_on_shared_header_line() {
    let edited = HELPER.replace("retrying", "again");
    let a = format!(
        "{HELPER}\nconst main = async (input) => {{\n  return api.post(\"/a\", input);\n}};\n"
    );
    let b = format!("{edited}\nconst main = async (input) => {{\n  logger.info(\"b\");\n}};\n");
    assert_eq!(
        clones(&[("a.ts", &a), ("b.ts", &b)], 12, 10),
        vec![(40, vec![(1, 15), (1, 15)])]
    );
}

#[test]
fn clone_diverging_inside_loop_ends_at_last_full_line() {
    let edited = RETRY_BODY.replace(
        "console.warn(\"retrying\", i, err);",
        "metrics.count(\"fail\");",
    );
    let a = format!("function one(fn, attempts) {{\n{RETRY_BODY}  throw lastError;\n}}\n");
    let b = format!("function two(fn, attempts) {{\n{edited}  throw lastError;\n}}\n");
    assert_eq!(
        clones(&[("a.ts", &a), ("b.ts", &b)], 15, 0),
        vec![(21, vec![(1, 7), (1, 7)])]
    );
}

#[test]
fn straddled_tokens_do_not_lift_a_helper_over_the_threshold() {
    let files = helper_then(
        "const loadUsers = async (client) => {\n  return client.query(\"users\");\n};\n",
        "const saveOrder = async (order) => {\n  return api.post(\"/orders\", order);\n};\n",
    );
    assert!(clones(&refs(&files), 38, 0).is_empty());
    assert_eq!(clones(&refs(&files), 37, 0).len(), 1);
}
