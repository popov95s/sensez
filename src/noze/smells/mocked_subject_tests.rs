//! When a test replaces its own subject with a double.

use super::test_support::local_of;
use crate::config::smells::{Smells, Strictness};
use crate::report::SmellKind;

fn mocked_lines(name: &str, source: &str, strictness: Strictness) -> Vec<usize> {
    let config = Smells {
        mocked_subject_strictness: strictness,
        ..Smells::default()
    };
    local_of(name, source, &config, SmellKind::MockedSubject)
        .into_iter()
        .map(|finding| finding.line)
        .collect()
}

fn ts(source: &str) -> Vec<usize> {
    mocked_lines("quote.test.ts", source, Strictness::Medium)
}

fn py(source: &str) -> Vec<usize> {
    mocked_lines("test_quote.py", source, Strictness::Medium)
}

#[test]
fn mocking_the_module_the_called_subject_comes_from_fires() {
    let source = r#"import { quote } from "./pricing";
vi.mock("./pricing", () => ({ quote: vi.fn(() => 42) }));
it("quotes", () => { expect(quote({ qty: 2 })).toBe(42); });
"#;
    assert_eq!(ts(source), vec![2]);
}

#[test]
fn mocking_a_collaborator_is_quiet() {
    let source = r#"import { quote } from "./pricing";
vi.mock("./rates", () => ({ fetchRate: vi.fn(async () => 1.5) }));
it("quotes", async () => { expect(await quote({ qty: 2 })).toBe(3); });
"#;
    assert_eq!(ts(source), Vec::<usize>::new());
}

#[test]
fn partial_mock_keeping_the_original_is_quiet() {
    let source = r#"import { quote } from "./pricing";
vi.mock("./pricing", async (importOriginal) => ({ ...(await importOriginal()), round: vi.fn() }));
it("quotes", () => { expect(quote({ qty: 2 })).toBe(4); });
"#;
    assert_eq!(ts(source), Vec::<usize>::new());
}

#[test]
fn asserting_on_a_mocked_collaborator_is_quiet() {
    let source = r#"import { send } from "./mailer";
import { notify } from "./notify";
vi.mock("./mailer");
it("notifies", () => {
  vi.mocked(send).mockReturnValue(true);
  send.mockClear();
  notify("a");
  expect(send).toHaveBeenCalled();
});
"#;
    assert_eq!(ts(source), Vec::<usize>::new());
}

#[test]
fn subject_called_inside_a_nested_callback_fires() {
    let source = r#"import * as pricing from "../shop/pricing";
jest.mock("../shop/pricing");
test("quotes", async () => {
  await act(async () => { pricing.quote(1); });
});
"#;
    assert_eq!(ts(source), vec![2]);
}

#[test]
fn mock_without_a_subject_import_is_quiet() {
    let source = r#"vi.mock("./pricing");
it("loads", async () => { const mod = await import("./pricing"); expect(mod).toBeDefined(); });
"#;
    assert_eq!(ts(source), Vec::<usize>::new());
}

#[test]
fn module_named_like_the_test_fires_only_at_high() {
    let source = r#"import { render } from "./render";
vi.mock("./quote");
it("renders", () => { expect(render()).toBe(""); });
"#;
    assert_eq!(ts(source), Vec::<usize>::new());
    assert_eq!(
        mocked_lines("quote.test.ts", source, Strictness::High),
        vec![2]
    );
}

#[test]
fn patching_the_called_subject_fires() {
    let source = r#"from unittest.mock import patch
from shop.pricing import quote

@patch("shop.pricing.quote", return_value=42)
def test_quote(mocked):
    assert quote(2) == 42
"#;
    assert_eq!(py(source), vec![4]);
}

#[test]
fn patching_a_collaborator_in_the_subject_module_is_quiet() {
    let source = r#"from shop.pricing import quote

def test_quote(mocker):
    mocker.patch("shop.pricing.fetch_rate", return_value=2)
    assert quote(2) == 4
"#;
    assert_eq!(py(source), Vec::<usize>::new());
}

#[test]
fn patching_through_a_module_binding_fires() {
    let source = r#"from shop import pricing

def test_quote(monkeypatch):
    monkeypatch.setattr("shop.pricing.quote", lambda qty: 42)
    assert pricing.quote(2) == 42
"#;
    assert_eq!(py(source), vec![4]);
}

#[test]
fn patch_with_wraps_is_quiet() {
    let source = r#"from unittest.mock import patch
from shop.pricing import quote

def test_quote():
    with patch("shop.pricing.quote", wraps=quote):
        assert quote(2) == 4
"#;
    assert_eq!(py(source), Vec::<usize>::new());
}

#[test]
fn patch_object_on_the_subject_fires_only_at_high() {
    let source = r#"from unittest.mock import patch
from shop.pricing import Pricing

def test_quote():
    with patch.object(Pricing, "rate", return_value=2):
        assert Pricing().quote(2) == 4
"#;
    assert_eq!(py(source), Vec::<usize>::new());
    assert_eq!(
        mocked_lines("test_quote.py", source, Strictness::High),
        vec![5]
    );
}

#[test]
fn factory_receiving_the_original_module_is_quiet() {
    let source = r#"import { quote } from "./pricing";
vi.mock("./pricing", async (real) => ({ ...(await real()), round: vi.fn() }));
it("quotes", () => { expect(quote({ qty: 2 })).toBe(4); });
"#;
    assert_eq!(ts(source), Vec::<usize>::new());
}

#[test]
fn same_named_module_in_another_directory_is_quiet_at_high() {
    let source = r#"import { render } from "./render";
vi.mock("../billing/quote");
it("renders", () => { expect(render()).toBe(""); });
"#;
    assert_eq!(
        mocked_lines("quote.test.ts", source, Strictness::High),
        Vec::<usize>::new()
    );
}

#[test]
fn inline_import_of_the_real_module_is_quiet() {
    let source = r#"vi.mock("./pricing");
it("quotes", async () => {
  const { quote } = await vi.importActual("./pricing");
  expect(quote(2)).toBe(4);
});
it("stubs", async () => {
  const { quote } = await import("./pricing");
  vi.mocked(quote).mockReturnValue(1);
});
"#;
    assert_eq!(ts(source), Vec::<usize>::new());
}
