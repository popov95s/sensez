use super::detect_local;
use crate::config::smells::Smells;
use crate::report::{SmellFinding, SmellKind};
use crate::spine::parser::parse_file;
use std::fs;

fn enabled() -> Smells {
    let mut cfg = Smells::default();
    cfg.disabled
        .retain(|kind| !matches!(kind, SmellKind::NestedLoop | SmellKind::NPlusOneCall));
    cfg
}

fn with_callees() -> Smells {
    Smells {
        n_plus_one_callees: ["*.db.*", "*.run*", "*Store.*"].map(String::from).to_vec(),
        ..enabled()
    }
}

fn run(ext: &str, body: &str, cfg: &Smells) -> Vec<SmellFinding> {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join(format!("sample.{ext}"));
    fs::write(&path, body).unwrap();
    detect_local(&parse_file(&path, 0).unwrap(), cfg)
}

fn fires(ext: &str, body: &str, cfg: &Smells, kind: SmellKind) -> bool {
    run(ext, body, cfg).iter().any(|f| f.kind == kind)
}

#[test]
fn sort_in_loop_skips_header_and_per_item_values() {
    for src in [
        "export function s1(items, cmp) { for (const x of items.sort(cmp)) use(x); }\n",
        "export function s2(groups) { for (const g of groups) g.items.sort(); }\n",
        "export function local(gs) { for (const g of gs) { const local = [...g.xs]; local.sort(); } }\n",
    ] {
        assert!(!fires("ts", src, &enabled(), SmellKind::SortInLoop), "{src}");
    }
}

#[test]
fn sort_in_loop_keeps_repeated_sort_of_outer_list() {
    let src = "export function tp(xs, acc) { for (const x of xs) { acc.push(x); acc.sort(); } }\n";
    assert!(fires("ts", src, &enabled(), SmellKind::SortInLoop));
}

#[test]
fn nested_loop_skips_walking_nested_data() {
    for src in [
        "export function walk(orders) { for (const o of orders) for (const l of o.lines) use(l); }\n",
        "export function groups(groups) { for (const [k, list] of Object.entries(groups)) for (const x of list) use(k, x); }\n",
        "export function rows(grid) { for (const row of grid) for (let j = 0; j < row.length; j++) use(row[j]); }\n",
        "export function cb(xs, ys) { xs.forEach((x) => { for (const y of ys) use(x, y); }); }\n",
    ] {
        assert!(!fires("ts", src, &enabled(), SmellKind::NestedLoop), "{src}");
    }
    let py = "def walk(orders):\n    for o in orders:\n        for line in o.lines:\n            print(line)\n";
    assert!(!fires("py", py, &enabled(), SmellKind::NestedLoop));
    let rs = "fn walk(orders: &[Order]) {\n    for o in orders {\n        for line in &o.lines {\n            use_line(line);\n        }\n    }\n}\n";
    assert!(!fires("rs", rs, &enabled(), SmellKind::NestedLoop));
}

#[test]
fn nested_loop_keeps_independent_and_self_joins() {
    let tp = "export function tp(as, bs) { for (const a of as) for (const b of bs) use(a, b); }\n";
    let finding = run("ts", tp, &enabled())
        .into_iter()
        .find(|f| f.kind == SmellKind::NestedLoop)
        .expect("independent nested loop must fire");
    assert!(
        finding
            .message
            .contains("inner loop over `bs` walks an independent collection for each item of `as`"),
        "{}",
        finding.message
    );
    let self_join =
        "export function pairs(xs) { for (const a of xs) for (const b of xs) use(a, b); }\n";
    assert!(fires("ts", self_join, &enabled(), SmellKind::NestedLoop));
    let field_name = "export function f(a, b) { for (const lines of a) for (const x of b.lines) use(lines, x); }\n";
    assert!(fires("ts", field_name, &enabled(), SmellKind::NestedLoop));
}

#[test]
fn n_plus_one_skips_header_bounded_and_paging_loops() {
    for src in [
        "export async function a5(ctx) { for (const row of await ctx.db.query(\"t\").collect()) use(row); }\n",
        "export async function a6(ctx) { let cursor = null; do { const page = await ctx.db.query(cursor); cursor = page.next; } while (cursor); }\n",
        "export async function more(api) { let more = true; while (more) { const res = await api.query(); more = res.hasMore; } }\n",
        "export async function token(api, token) { while (token) { token = (await api.query(token)).next; } }\n",
        "export async function a7(api) { for (const s of [\"a\", \"b\", \"c\"]) await api.query(s); }\n",
        "export async function a8(client, x) { for (let attempt = 0; attempt < 3; attempt++) { try { return await client.request(x); } catch {} } }\n",
        "export async function cached(ids, cache) { for (const id of ids) cache.get(id); }\n",
    ] {
        assert!(!fires("ts", src, &with_callees(), SmellKind::NPlusOneCall), "{src}");
    }
    let py = "def page(api, cursor):\n    while cursor:\n        page = api.query(cursor)\n        cursor = page.next\n";
    assert!(!fires("py", py, &enabled(), SmellKind::NPlusOneCall));
    let header = "def rows(db):\n    for row in db.query(\"t\"):\n        print(row)\n";
    assert!(!fires("py", header, &enabled(), SmellKind::NPlusOneCall));
    let retry =
        "fn retry(client: &Client) {\n    for _ in 0..3 {\n        client.request();\n    }\n}\n";
    assert!(!fires("rs", retry, &enabled(), SmellKind::NPlusOneCall));
}

#[test]
fn n_plus_one_keeps_per_item_calls() {
    for src in [
        "export async function tp1(api, ids) { for (const id of ids) await api.query(id); }\n",
        "export async function counter(db, xs, n) { let i = 0; while (i < n) { await db.query(xs[i]); i = i + 1; } }\n",
        "export async function bump(db, xs, n) { let i = 0; do { await db.query(xs[i]); i++; } while (i < n); }\n",
        "export async function work(api, queue) { while (queue.length) { const item = queue.pop(); await api.query(item); } }\n",
        "export async function many(api) { for (let i = 0; i < 20; i++) await api.query(i); }\n",
    ] {
        assert!(fires("ts", src, &enabled(), SmellKind::NPlusOneCall), "{src}");
    }
    let wide = Smells {
        n_plus_one_max_constant_bound: 20,
        ..enabled()
    };
    let src =
        "export async function many(api) { for (let i = 0; i < 20; i++) await api.query(i); }\n";
    assert!(!fires("ts", src, &wide, SmellKind::NPlusOneCall));
}

#[test]
fn n_plus_one_callees_catch_data_layer_calls() {
    for src in [
        "export async function a1(ctx, ids) { for (const id of ids) await ctx.db.get(id); }\n",
        "export async function a2(ctx, ids, ref) { for (const id of ids) await ctx.runQuery(ref, { id }); }\n",
        "export async function a3(ctx, ids) { await Promise.all(ids.map((id) => ctx.db.patch(id, {}))); }\n",
        "export async function a4(ctx, ids) { for (const id of ids) await itemsStore.getById(ctx, id); }\n",
    ] {
        assert!(fires("ts", src, &with_callees(), SmellKind::NPlusOneCall), "{src}");
        assert!(!fires("ts", src, &enabled(), SmellKind::NPlusOneCall), "{src}");
    }
}

#[test]
fn await_in_loop_flags_awaited_calls_on_outside_receivers() {
    let cfg = Smells {
        n_plus_one_await_in_loop: true,
        ..enabled()
    };
    let outside =
        "export async function f(svc, ids) { for (const id of ids) await svc.hydrate(id); }\n";
    assert!(fires("ts", outside, &cfg, SmellKind::NPlusOneCall));
    assert!(!fires("ts", outside, &enabled(), SmellKind::NPlusOneCall));
    let own = "export async function f(users) { for (const u of users) await u.hydrate(); }\n";
    assert!(!fires("ts", own, &cfg, SmellKind::NPlusOneCall));
}
