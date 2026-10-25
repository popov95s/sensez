use super::test_support::local_of;
use crate::config::smells::Smells;
use crate::report::{SmellFinding, SmellKind};

fn repeated(name: &str, body: &str) -> Vec<SmellFinding> {
    local_of(name, body, &Smells::default(), SmellKind::RepeatedIteration)
}

#[test]
fn complementary_filter_pair_is_quiet() {
    let src = "export function split(files: F[]) {\n  return {\n    ok: files.filter((f) => isOk(f)),\n    bad: files.filter((f) => !isOk(f)),\n  };\n}\n";
    assert!(repeated("split.ts", src).is_empty());
}

#[test]
fn index_built_from_a_pass_does_not_count() {
    let src = "export function rows(seats: S[]) {\n  const known = new Set(seats.map((s) => s.id + 1));\n  const total = seats.reduce((n, s) => n + s.size, 0);\n  return { known, total };\n}\n";
    assert!(repeated("rows.ts", src).is_empty());
}

#[test]
fn two_transforms_without_an_aggregate_are_quiet() {
    let src = "export function shape(xs: X[]) {\n  const ids = xs.map((x) => x.id + 1);\n  const live = xs.filter((x) => x.live);\n  return [ids, live];\n}\n";
    assert!(repeated("shape.ts", src).is_empty());
}

#[test]
fn counted_filter_next_to_a_reduce_fires() {
    let src = "export function sum(items: I[]) {\n  const paid = items.filter((i) => i.paid).length;\n  const total = items.reduce((s, i) => s + i.price, 0);\n  return { paid, total };\n}\n";
    assert_eq!(repeated("sum.ts", src).len(), 1);
}

#[test]
fn some_every_and_reduce_over_one_array_fire() {
    let src = "export function stats(rows: R[]) {\n  const a = rows.some((r) => r.on);\n  const b = rows.every((r) => r.on);\n  const n = rows.reduce((s, r) => s + r.n, 0);\n  return { a, b, n };\n}\n";
    assert_eq!(repeated("stats.ts", src).len(), 1);
}

#[test]
fn two_for_of_loops_over_one_array_fire() {
    let src = "export function loops(rows: R[]) {\n  let total = 0;\n  for (const r of rows) total += r.n;\n  let on = 0;\n  for (const r of rows) if (r.on) on += 1;\n  return { total, on };\n}\n";
    let found = repeated("loops.ts", src);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].message.contains("`rows` is iterated 2 times"),
        "{}",
        found[0].message
    );
}

#[test]
fn second_loop_reading_a_value_the_first_produced_is_quiet() {
    let src = "export function spread(rows: R[]) {\n  let max = 0;\n  for (const r of rows) max = Math.max(max, r.n);\n  const top = max * 2;\n  let gap = 0;\n  for (const r of rows) gap += top - r.n;\n  return gap;\n}\n";
    assert!(repeated("spread.ts", src).is_empty());
}

#[test]
fn python_generator_passes_name_the_iterated_collection() {
    let src = "def summary(rows):\n    paid = len([r for r in rows if r.paid])\n    total = sum(r.price for r in rows)\n    top = max(r.price for r in rows)\n    return paid, total, top\n";
    let found = repeated("summary.py", src);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].message.contains("`rows` is iterated 3 times"),
        "{}",
        found[0].message
    );
}

#[test]
fn python_min_and_max_pair_is_quiet() {
    let src = "def bounds(values):\n    return min(values), max(values)\n";
    assert!(repeated("bounds.py", src).is_empty());
}

#[test]
fn python_partition_comprehensions_are_quiet() {
    let src = "def split(files):\n    ok = [f for f in files if f.ok]\n    bad = [f for f in files if not f.ok]\n    return ok, bad\n";
    assert!(repeated("split.py", src).is_empty());
}

#[test]
fn total_next_to_a_rendered_list_is_quiet() {
    let src = "export const Scores = (p: { scores: S[] }) => {\n  const total = p.scores.reduce((n, s) => n + s.value, 0);\n  return <ul>{total}{p.scores.map((s) => (<li key={s.id}>{s.value}</li>))}</ul>;\n};\n";
    assert!(repeated("scores.tsx", src).is_empty());
}

#[test]
fn loop_next_to_an_id_projection_is_quiet() {
    let src = "export async function move(rows: R[]) {\n  for (const r of rows) await save(r);\n  return rows.map((r) => r.id);\n}\n";
    assert!(repeated("move.ts", src).is_empty());
}

#[test]
fn loop_next_to_a_transform_is_quiet() {
    let src = "export function blocks(rows: R[]) {\n  const out = rows.map((r, i) => ({ i, text: r.text.trim() }));\n  for (const r of rows) log(r);\n  return out;\n}\n";
    assert!(repeated("blocks.ts", src).is_empty());
}
