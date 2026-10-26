use crate::config::model::Duplication;
use crate::noze::duplication::test_support::{pairs, write_files};
use crate::report::CloneClass;

const JS_SUMMARY: &str = "export function summarize(orders, limit) {
  const totals = new Map();
  for (const order of orders) {
    const key = order.customer.id;
    const entry = totals.get(key) || { count: 0, amount: 0 };
    entry.count += 1;
    entry.amount += order.total * order.quantity;
    totals.set(key, entry);
  }
  const ranked = Array.from(totals.entries()).sort((a, b) => b[1].amount - a[1].amount);
  if (ranked.length > limit) {
    return ranked.slice(0, limit);
  }
  return ranked;
}
";

const TS_SUMMARY: &str = "type Line = { customer: { id: string }; total: number; quantity: number };

export function summarize<T extends Line>(orders: T[], limit: 10 | 20): Array<[string, { count: number }]> {
  const totals = new Map<string, { count: number; amount: number }>();
  for (const order of orders) {
    const key: typeof order.customer.id = order.customer.id;
    const entry = totals.get(key) || ({ count: 0, amount: 0 } satisfies { count: number; amount: number });
    entry.count += 1;
    entry.amount += order.total * (order.quantity as Line[\"quantity\"]);
    totals.set(key, entry);
  }
  const ranked = Array.from(totals.entries()).sort((a: [string, { amount: number }], b: [string, { amount: number }]) => b[1].amount - a[1].amount);
  if (ranked.length > limit) {
    return ranked.slice(0, limit);
  }
  return ranked;
}
";

const JSX_CARD: &str = "export function Card({ title, items, onSelect }) {
  const visible = items.filter((item) => item.enabled && item.label.length > 0);
  const summary = visible.length > 0 ? `${visible.length} of ${items.length}` : \"none\";
  return (
    <section className=\"card\">
      <h2>{title}</h2>
      <p className=\"summary\">{summary}</p>
      <ul>
        {visible.map((item) => (
          <li key={item.id} onClick={() => onSelect(item.id)}>{item.label}</li>
        ))}
      </ul>
    </section>
  );
}
";

fn detect(files: &[(&str, &str)], max_gap: usize) -> Vec<CloneClass> {
    let tmp = tempfile::tempdir().unwrap();
    let parsed = write_files(tmp.path(), files);
    let config = Duplication {
        exclude: vec![],
        max_gap,
        ..Default::default()
    };
    crate::noze::duplication::detect(&parsed, &config)
}

fn detect_default(files: &[(&str, &str)]) -> Vec<CloneClass> {
    detect(files, Duplication::default().max_gap)
}

#[test]
fn ts_and_mjs_copies_are_one_clone_family() {
    let clones = detect_default(&[("util.ts", JS_SUMMARY), ("script.mjs", JS_SUMMARY)]);
    assert!(pairs(&clones, "util.ts", "script.mjs"), "{clones:?}");
}

#[test]
fn typed_ts_matches_untyped_js() {
    // Exact matching only, so stitching cannot bridge annotation tokens.
    let clones = detect(&[("util.ts", TS_SUMMARY), ("script.js", JS_SUMMARY)], 0);
    assert!(pairs(&clones, "util.ts", "script.js"), "{clones:?}");
}

#[test]
fn cjs_matches_ts() {
    let clones = detect_default(&[("util.ts", TS_SUMMARY), ("legacy.cjs", JS_SUMMARY)]);
    assert!(pairs(&clones, "util.ts", "legacy.cjs"), "{clones:?}");
}

#[test]
fn jsx_matches_tsx() {
    let clones = detect_default(&[("Card.jsx", JSX_CARD), ("Card2.tsx", JSX_CARD)]);
    assert!(pairs(&clones, "Card.jsx", "Card2.tsx"), "{clones:?}");
}

fn page_names() -> Vec<String> {
    (0..30).map(|i| format!("Page{i}")).collect()
}

fn interface_map() -> String {
    let fields: String = page_names()
        .iter()
        .map(|name| format!("  {name}: typeof {name};\n"))
        .collect();
    format!("export interface PageMap {{\n{fields}}}\n")
}

fn value_map() -> String {
    let fields: String = page_names()
        .iter()
        .map(|name| format!("  {name}: {name},\n"))
        .collect();
    format!("export const pageMap = {{\n{fields}}};\n")
}

#[test]
fn interface_map_does_not_match_value_map() {
    let clones = detect_default(&[("types.ts", &interface_map()), ("values.ts", &value_map())]);
    assert!(!pairs(&clones, "types.ts", "values.ts"), "{clones:?}");
}

#[test]
fn identical_interfaces_and_identical_value_maps_clone_separately() {
    let types = interface_map();
    let values = value_map();
    let clones = detect_default(&[
        ("types1.ts", &types),
        ("types2.ts", &types),
        ("values1.ts", &values),
        ("values2.ts", &values),
    ]);
    assert!(pairs(&clones, "types1.ts", "types2.ts"), "{clones:?}");
    assert!(pairs(&clones, "values1.ts", "values2.ts"), "{clones:?}");
    assert!(!pairs(&clones, "types1.ts", "values1.ts"), "{clones:?}");
}
