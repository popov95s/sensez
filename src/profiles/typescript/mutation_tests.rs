//! TypeScript `mutated_parameter` and `literal_membership` precision.

use crate::config::smells::Smells;
use crate::noze::smells::detect_local;
use crate::noze::smells::test_support as fixture;
use crate::report::{Severity, SmellFinding, SmellKind};

fn findings(ext: &str, body: &str, cfg: &Smells, kind: SmellKind) -> Vec<SmellFinding> {
    detect_local(&fixture::parsed(&format!("m.{ext}"), body), cfg)
        .into_iter()
        .filter(|f| f.kind == kind)
        .collect()
}

fn mutated(body: &str) -> Vec<SmellFinding> {
    findings("ts", body, &Smells::default(), SmellKind::MutatedParameter)
}

fn membership(ext: &str, body: &str, cfg: &Smells) -> Vec<SmellFinding> {
    findings(ext, body, cfg, SmellKind::LiteralMembership)
}

#[test]
fn callbacks_and_typed_handles_are_not_mutated_parameters() {
    let body = "\
interface Logger { set(k: string, v: number): void }
interface Client { delete(): Promise<void> }
export function index(items: { k: string }[]) {
  return items.reduce((acc, x) => { acc[x.k] = x; return acc; }, {});
}
export function sortGroups() {
  const groups = [[2, 1], [4, 3]];
  groups.forEach((g) => g.sort());
  return groups;
}
export function logIt(event: Logger) { event.set(\"k\", 1); }
export async function teardown(sandbox: Client) { await sandbox.delete(); }
";
    let found = mutated(body);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn collection_mutation_of_inputs_still_flags() {
    let body = "\
export function sortInPlace(values: number[]) { values.sort(); return values; }
export function addAll(target: Set<string>, xs: string[]) {
  for (const x of xs) { target.add(x); }
}
";
    let found = mutated(body);
    let symbols: Vec<&str> = found.iter().map(|f| f.symbol.as_str()).collect();
    assert_eq!(symbols, vec!["sortInPlace", "addAll"]);
    assert_eq!(found[0].severity, Severity::Warning);
    assert!(found[0].message.contains("mutates and returns its input"));
    assert_eq!(found[1].severity, Severity::Warning);

    let js = findings(
        "js",
        "export function f(list) { list.push(1); }\n",
        &Smells::default(),
        SmellKind::MutatedParameter,
    );
    assert_eq!(js.len(), 1, "unannotated JS params keep firing");
}

#[test]
fn private_output_parameter_is_info() {
    let body = "\
interface Node { name: string; children: Node[] }
function walk(node: Node, out: string[]): void {
  out.push(node.name);
}
export function names(root: Node): string[] { const out: string[] = []; walk(root, out); return out; }
";
    let found = mutated(body);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].symbol, "walk");
    assert_eq!(found[0].severity, Severity::Info);
    assert!(found[0].message.contains("private collectors"));
}

#[test]
fn external_vocabularies_and_literal_typed_operands_are_not_categories() {
    let body = "\
type Status = \"draft\" | \"sent\" | \"closed\";
const IMAGE_EXTENSIONS = new Set([\".png\"]);
export function isImage(ext: string, name: string) {
  return [\".png\", \".jpg\"].includes(ext) || IMAGE_EXTENSIONS.has(name);
}
export function isMissing(err: { code: string }) { return [\"ENOENT\", \"EACCES\"].includes(err.code); }
export function isOpen(status: Status) { return [\"draft\", \"sent\"].includes(status); }
";
    assert!(membership("ts", body, &Smells::default()).is_empty());

    let keep_vocab = Smells {
        literal_membership_skip_external_vocab: false,
        ..Smells::default()
    };
    let symbols: Vec<String> = membership("ts", body, &keep_vocab)
        .into_iter()
        .map(|f| f.symbol)
        .collect();
    assert_eq!(symbols, vec!["isImage", "isMissing"]);
}

#[test]
fn inline_category_lists_flag_with_typescript_advice() {
    let body = "\
export function route(kind: string, role: string, tier: string) {
  if ([\"admin\", \"owner\"].includes(kind)) { return 1; }
  if (new Set([\"admin\", \"owner\"]).has(role)) { return 2; }
  return ([\"gold\", \"silver\"] as const).includes(tier as \"gold\") ? 3 : 0;
}
";
    let found = membership("ts", body, &Smells::default());
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].metric, 3);
    assert_eq!(found[0].lines, vec![2, 3, 4]);
    assert!(
        found[0].message.contains("string-literal union"),
        "{}",
        found[0].message
    );

    let py = membership(
        "py",
        "def f(x):\n    return x in [\"a\", \"b\"]\n",
        &Smells::default(),
    );
    assert_eq!(py.len(), 1);
    assert!(py[0].message.contains("an Enum"));
}
