//! Tests for the Python traversal/import extraction (kept out of `traversal.rs`
//! to respect the per-file line limit).

use crate::spine::parser::{parse_source, StructuralToken};

fn tokens(src: &[u8]) -> Vec<StructuralToken> {
    parse_source(src, 0, "m", &super::super::PythonProfile)
        .unwrap()
        .syntax
        .tokens
}

fn imports(src: &[u8]) -> Vec<crate::spine::parser::ImportContext> {
    parse_source(src, 0, "m", &super::super::PythonProfile)
        .unwrap()
        .symbols
        .imports
}

/// Two functions with identical control structure but entirely different
/// identifier/literal names produce identical token vectors.
#[test]
fn rename_invariance_yields_identical_tokens() {
    let a = b"def compute(items):\n    total = 0\n    for item in items:\n        if item > 10:\n            total = total + item\n    return total\n";
    let b = b"def process(values):\n    acc = 99\n    for value in values:\n        if value > 500:\n            acc = acc + value\n    return acc\n";

    let ta = tokens(a);
    let tb = tokens(b);
    assert_eq!(
        ta, tb,
        "renamed-but-structurally-identical functions must match"
    );
    assert!(ta.contains(&StructuralToken::FunctionDef));
    assert!(ta.contains(&StructuralToken::ForStatement));
    assert!(ta.contains(&StructuralToken::IfStatement));
    assert!(ta.contains(&StructuralToken::Return));
}

/// A top-level import and a nested inline import are distinguished.
#[test]
fn inline_vs_top_level_imports_distinguished() {
    let imports = imports(b"import os\n\ndef loader():\n    import json\n    return json\n");
    assert_eq!(imports.len(), 2);

    let top = imports.iter().find(|i| i.target_module == "os").unwrap();
    assert!(!top.is_inline);
    assert_eq!(top.enclosing_scope, None);
    assert_eq!(top.line, 1);

    let inline = imports.iter().find(|i| i.target_module == "json").unwrap();
    assert!(inline.is_inline);
    assert_eq!(inline.enclosing_scope.as_deref(), Some("loader"));
    assert_eq!(inline.line, 4);
}

/// `from x import a, b` collects the symbol list on one context.
#[test]
fn from_import_collects_symbols() {
    let imports = imports(b"from app.models import User, Order\n");
    assert_eq!(imports.len(), 1);
    assert_eq!(imports[0].target_module, "app.models");
    assert_eq!(imports[0].imported_symbols, vec!["User", "Order"]);
}

/// `bindings` records the *local* name an import introduces (alias-aware).
#[test]
fn imports_track_alias_aware_bindings() {
    let osp = imports(b"import os.path as osp\n");
    assert_eq!(osp[0].target_module, "os.path");
    assert_eq!(osp[0].bindings, vec!["osp"]);

    let frm = imports(b"from app.models import User as U, Order\n");
    assert_eq!(frm[0].imported_symbols, vec!["User", "Order"]);
    assert_eq!(frm[0].bindings, vec!["U", "Order"]);

    let plain = imports(b"import a.b.c\n");
    assert_eq!(plain[0].bindings, vec!["a"]);
}

/// An import nested inside a method reports the *nearest* enclosing scope.
#[test]
fn nested_inline_import_uses_nearest_scope() {
    let imports =
        imports(b"class C:\n    def handler(self):\n        import json\n        return json\n");
    let json = imports.iter().find(|i| i.target_module == "json").unwrap();
    assert!(json.is_inline);
    assert_eq!(json.enclosing_scope.as_deref(), Some("handler"));
    assert_eq!(json.line, 3);
}

fn walked(src: &[u8]) -> crate::spine::parser::Walked {
    parse_source(src, 0, "m", &super::super::PythonProfile).unwrap()
}

/// REGRESSION GUARD for the duplication pillar: docstrings and comments must
/// never enter the structural token/lexeme stream (clone detection ignores
/// them). The same function with vs without a docstring + comment must yield
/// identical `tokens` and `lexemes`. If a future change let a docstring fall
/// through to `map_kind`, this fails — exactly the regression we must prevent.
#[test]
fn docstrings_and_comments_never_enter_token_or_lexeme_stream() {
    let with_docs =
        b"def f(x):\n    \"\"\"Add one to x.\"\"\"\n    # increment it\n    return x + 1\n";
    let without = b"def f(x):\n    return x + 1\n";

    let a = walked(with_docs);
    let b = walked(without);
    assert_eq!(
        a.syntax.tokens, b.syntax.tokens,
        "docstring/comment must add no tokens"
    );
    assert_eq!(
        a.syntax.lexemes, b.syntax.lexemes,
        "docstring/comment must add no lexemes"
    );
    assert_eq!(
        a.syntax.lexemes.len(),
        a.syntax.tokens.len(),
        "lexemes stay 1:1 with tokens"
    );
}

/// Under `eyez`, docstrings + comments are captured into `Walked.docs` and
/// mapped to their fully-qualified symbol paths — without disturbing the above.
#[cfg(feature = "eyez")]
#[test]
fn captures_docs_with_qualified_symbol_paths() {
    use crate::eyez::DocKind;
    let src = b"\"\"\"Module doc.\"\"\"\n# top comment\nclass Billing:\n    \"\"\"Bills things.\"\"\"\n    def verify(self):\n        \"\"\"Verify sig.\"\"\"\n        return 1\n";
    let docs = walked(src).docs;

    let has = |path: &str, kind: DocKind, text: &str| {
        docs.iter()
            .any(|d| d.symbol_path == path && d.kind == kind && d.text == text)
    };
    assert!(
        has("m", DocKind::Docstring, "Module doc."),
        "module docstring"
    );
    assert!(has("m", DocKind::Comment, "top comment"), "module comment");
    assert!(
        has("m::Billing", DocKind::Docstring, "Bills things."),
        "class"
    );
    assert!(
        has("m::Billing.verify", DocKind::Docstring, "Verify sig."),
        "method docstring qualified under class"
    );
}
