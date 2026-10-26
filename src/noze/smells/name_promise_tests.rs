use crate::config::smells::{Smells, Strictness};
use crate::noze::smells::test_support::local_of;
use crate::report::{SmellFinding, SmellKind};

fn hits_with(name: &str, source: &str, config: &Smells) -> Vec<SmellFinding> {
    local_of(name, source, config, SmellKind::NamePromiseMismatch)
}

fn hits(name: &str, source: &str) -> Vec<SmellFinding> {
    hits_with(name, source, &Smells::default())
}

fn at(strictness: Strictness) -> Smells {
    Smells {
        name_promise_strictness: strictness,
        ..Smells::default()
    }
}

const LAZY_TS: &str =
    "let conn;\nexport function getConn() {\n  if (!conn) conn = open();\n  return conn;\n}\n";

#[test]
fn database_insert_in_a_getter_is_reported() {
    let source = "export async function getInvoice(db, id) {\n  const row = await db.invoices.find(id);\n  if (!row) await db.invoices.insert({ id });\n  return row;\n}\n";
    let found = hits("billing.ts", source);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].lines, vec![3]);
}

#[test]
fn message_names_the_write_and_its_line() {
    let source =
        "export async function getInvoice(repo, id) {\n  await repo.save(id, { seen: true });\n}\n";
    let found = hits("billing.ts", source);
    assert!(found[0]
        .message
        .contains("`getInvoice` writes: `repo.save` at L2"));
}

#[test]
fn names_that_admit_the_write_are_quiet() {
    for name in [
        "getOrCreateInvoice",
        "ensureInvoice",
        "checkAndStoreInvoice",
    ] {
        let source = format!(
            "export async function {name}(db, id) {{\n  await db.invoices.insert({{ id }});\n}}\n"
        );
        assert!(hits("billing.ts", &source).is_empty(), "{name}");
    }
}

#[test]
fn pushing_to_a_local_list_is_quiet() {
    let source = "export function listTotals(rows) {\n  const out = [];\n  rows.forEach((row) => out.push(row.total));\n  return out;\n}\n";
    assert!(hits("billing.ts", source).is_empty());
}

#[test]
fn closure_accumulator_of_the_enclosing_function_is_quiet() {
    let source = "export function makeClient() {\n  const seen = [];\n  return {\n    getNext: (id) => { seen.push(id); return id; },\n  };\n}\n";
    assert!(hits("billing.ts", source).is_empty());
}

#[test]
fn memo_cache_lookup_then_set_is_quiet() {
    let source = "const prices = new Map();\nexport function getPrice(sku) {\n  if (prices.has(sku)) return prices.get(sku);\n  const price = sku.length;\n  prices.set(sku, price);\n  return price;\n}\n";
    assert!(hits("billing.ts", source).is_empty());
}

#[test]
fn memo_cache_lookup_then_set_is_reported_at_high() {
    let source = "const prices = new Map();\nexport function getPrice(sku) {\n  if (prices.has(sku)) return prices.get(sku);\n  prices.set(sku, 1);\n  return 1;\n}\n";
    assert_eq!(
        hits_with("billing.ts", source, &at(Strictness::High)).len(),
        1
    );
}

#[test]
fn lazy_initialisation_is_quiet() {
    assert!(hits("conn.ts", LAZY_TS).is_empty());
}

#[test]
fn lazy_initialisation_is_reported_at_high() {
    assert_eq!(
        hits_with("conn.ts", LAZY_TS, &at(Strictness::High)).len(),
        1
    );
}

#[test]
fn module_name_assignment_is_reported() {
    let source = "let current;\nexport function getCurrent(next) {\n  current = next;\n  return current;\n}\n";
    assert_eq!(hits("state.ts", source).len(), 1);
}

#[test]
fn own_member_mutation_is_reported() {
    let source = "class Queue {\n  getHead() {\n    this.items.splice(0, 1);\n    return this.items[0];\n  }\n}\n";
    assert_eq!(hits("queue.ts", source).len(), 1);
}

#[test]
fn non_get_fetch_is_reported() {
    let source = "export async function findUser(id) {\n  await fetch(\"/users\", { method: \"POST\", body: id });\n}\n";
    assert_eq!(hits("users.ts", source).len(), 1);
}

#[test]
fn get_fetch_is_quiet() {
    let source = "export async function findUser(id) {\n  return fetch(\"/users\", { method: \"GET\" });\n}\n";
    assert!(hits("users.ts", source).is_empty());
}

#[test]
fn write_inside_an_inline_callback_is_reported() {
    let source = "export async function listSeen(store, ids) {\n  await Promise.all(ids.map((id) => store.users.update(id, { seen: true })));\n}\n";
    assert_eq!(hits("users.ts", source).len(), 1);
}

#[test]
fn enqueuing_a_job_in_a_getter_is_reported() {
    let source = "export async function getReport(queue, id) {\n  await queue.enqueue(\"build-report\", { id });\n}\n";
    assert_eq!(hits("reports.ts", source).len(), 1);
}

#[test]
fn validate_ignores_in_memory_error_lists() {
    let source = "class Form {\n  validateName(name) {\n    if (!name) this.errors.push(\"name\");\n  }\n}\n";
    assert!(hits("form.ts", source).is_empty());
}

#[test]
fn validate_reports_persistent_writes() {
    let source = "export async function validateOrder(repo, id) {\n  await repo.save(id, { checked: true });\n}\n";
    assert_eq!(hits("order.ts", source).len(), 1);
}

#[test]
fn accessors_are_quiet() {
    let source =
        "class Counter {\n  get isEmpty() {\n    this.reads.push(1);\n    return true;\n  }\n}\n";
    assert!(hits("counter.ts", source).is_empty());
}

#[test]
fn test_sources_are_quiet() {
    let source =
        "export async function getInvoice(db, id) {\n  await db.invoices.insert({ id });\n}\n";
    assert!(hits("billing.test.ts", source).is_empty());
}

#[test]
fn low_strictness_skips_in_memory_writes() {
    let source = "class Queue {\n  getHead() {\n    this.items.splice(0, 1);\n    return this.items[0];\n  }\n}\n";
    assert!(hits_with("queue.ts", source, &at(Strictness::Low)).is_empty());
}

#[test]
fn high_strictness_treats_load_as_a_read() {
    let source = "export async function loadUser(db, id) {\n  await db.users.insert({ id });\n}\n";
    assert!(hits("users.ts", source).is_empty());
    assert_eq!(
        hits_with("users.ts", source, &at(Strictness::High)).len(),
        1
    );
}

#[test]
fn read_prefixes_knob_adds_a_prefix() {
    let source = "export async function peekUser(db, id) {\n  await db.users.insert({ id });\n}\n";
    let mut config = Smells::default();
    config.name_promise_read_prefixes.push("peek".to_string());
    assert_eq!(hits_with("users.ts", source, &config).len(), 1);
}

#[test]
fn write_methods_knob_replaces_the_vocabulary() {
    let source = "export async function getUser(db, id) {\n  await db.users.insert({ id });\n}\n";
    let config = Smells {
        name_promise_write_methods: vec!["archive".to_string()],
        ..Smells::default()
    };
    assert!(hits_with("users.ts", source, &config).is_empty());
}

#[test]
fn python_self_member_assignment_is_reported() {
    let source =
        "class Account:\n    def get_balance(self):\n        self.reads = 1\n        return 0\n";
    assert_eq!(hits("account.py", source).len(), 1);
}

#[test]
fn python_global_assignment_is_reported() {
    let source =
        "_state = 0\n\ndef is_ready():\n    global _state\n    _state = 1\n    return True\n";
    assert_eq!(hits("ready.py", source).len(), 1);
}

#[test]
fn python_local_rebinding_is_quiet() {
    let source = "def get_total(rows):\n    total = 0\n    out = []\n    for row in rows:\n        out.append(row)\n        row.seen = True\n    return total\n";
    assert!(hits("totals.py", source).is_empty());
}

#[test]
fn python_lazy_member_is_quiet() {
    let source = "class Api:\n    def get_client(self):\n        if self._client is None:\n            self._client = object()\n        return self._client\n";
    assert!(hits("api.py", source).is_empty());
}
