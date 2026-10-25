use crate::config::smells::Smells;
use crate::noze::smells::test_support::local_of;
use crate::report::SmellKind;

fn stubs(name: &str, body: &str) -> Vec<String> {
    stubs_with(name, body, &Smells::default())
}

fn stubs_with(name: &str, body: &str, config: &Smells) -> Vec<String> {
    local_of(name, body, config, SmellKind::StubShipped)
        .into_iter()
        .map(|finding| finding.symbol)
        .collect()
}

#[test]
fn ts_throw_not_implemented_is_reported() {
    let source = "export function renderInvoice(id: string): string {\n  throw new Error(\"Not implemented yet\");\n}\n";
    assert_eq!(stubs("billing.ts", source), ["renderInvoice"]);
}

#[test]
fn ts_method_throwing_todo_is_reported() {
    let source = "export class Mailer {\n  send(to: string): void {\n    throw new Error(\"TODO\");\n  }\n}\n";
    assert_eq!(stubs("mailer.ts", source), ["send"]);
}

#[test]
fn ts_throw_with_an_ordinary_message_is_quiet() {
    let source =
        "export function loadTodos(): never {\n  throw new Error(\"failed to load todos\");\n}\n";
    assert!(stubs("todos.ts", source).is_empty());
}

#[test]
fn ts_empty_body_under_a_todo_is_reported() {
    let source = "export function syncCalendar(): void {\n  // TODO: wire the calendar sync\n}\n";
    assert_eq!(stubs("calendar.ts", source), ["syncCalendar"]);
}

#[test]
fn ts_return_null_under_a_fixme_is_reported() {
    let source = "export function pickTheme(): string | null {\n  // FIXME choose from settings\n  return null;\n}\n";
    assert_eq!(stubs("theme.ts", source), ["pickTheme"]);
}

#[test]
fn ts_empty_body_without_a_todo_is_quiet() {
    let source = "export function noop(): void {}\n";
    assert!(stubs("noop.ts", source).is_empty());
}

#[test]
fn ts_abstract_and_overload_declarations_are_quiet() {
    let source = "export abstract class Shape {\n  abstract area(): number;\n}\nexport function parse(x: string): number;\nexport function parse(x: string): number {\n  return Number(x);\n}\n";
    assert!(stubs("shape.ts", source).is_empty());
}

#[test]
fn custom_markers_replace_the_defaults() {
    let source = "export function plan(): void {\n  throw new Error(\"coming soon\");\n}\n";
    let mut config = Smells::default();
    assert!(stubs_with("plan.ts", source, &config).is_empty());
    config.stub_markers = vec!["coming soon".to_string()];
    assert_eq!(stubs_with("plan.ts", source, &config), ["plan"]);
}

#[test]
fn python_free_function_raising_not_implemented_is_reported() {
    let source = "def export_ledger(org_id):\n    raise NotImplementedError\n";
    assert_eq!(stubs("ledger.py", source), ["export_ledger"]);
}

#[test]
fn python_free_function_with_docstring_and_pass_is_reported() {
    let source = "def archive(item):\n    \"\"\"Move the item to cold storage.\"\"\"\n    pass\n";
    assert_eq!(stubs("archive.py", source), ["archive"]);
}

#[test]
fn python_abstract_method_is_quiet() {
    let source = "from abc import ABC, abstractmethod\n\nclass Exporter(ABC):\n    @abstractmethod\n    def export(self, org_id: str) -> bytes: ...\n";
    assert!(stubs("exporter.py", source).is_empty());
}

#[test]
fn python_overload_signature_is_quiet() {
    let source = "from typing import overload\n\n@overload\ndef scale(x: int) -> int: ...\ndef scale(x):\n    return x * 2\n";
    assert!(stubs("scale.py", source).is_empty());
}

#[test]
fn python_protocol_member_is_quiet() {
    let source = "from typing import Protocol\n\nclass Store(Protocol):\n    def get(self, key: str) -> str:\n        # TODO document\n        ...\n";
    assert!(stubs("store.py", source).is_empty());
}

#[test]
fn python_method_hook_without_a_todo_is_quiet() {
    let source = "class Job:\n    def on_finish(self):\n        pass\n";
    assert!(stubs("job.py", source).is_empty());
}

#[test]
fn python_method_under_a_todo_is_reported() {
    let source = "class Job:\n    def retry(self):\n        # TODO: back off and retry\n        raise NotImplementedError\n";
    assert_eq!(stubs("job.py", source), ["retry"]);
}

#[test]
fn python_stub_file_is_quiet() {
    let source = "def connect(url: str) -> object: ...\n";
    assert!(stubs("client.pyi", source).is_empty());
}

#[test]
fn conventional_placeholder_helper_is_quiet() {
    let source = "export function unimplemented(feature: string): never {\n  throw new Error(`unimplemented: ${feature}`);\n}\n";
    assert!(stubs("guards.ts", source).is_empty());
}
