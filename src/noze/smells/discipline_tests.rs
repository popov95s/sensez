//! Tests for the type/mutation-discipline detectors (`typing.rs`, `mutation.rs`).

use super::*;
use crate::config::smells::Smells;
use crate::spine::parser::parse_file;
use std::fs;

/// Write `body` to a temp `.py` file, parse it, and run the local detectors.
fn local(name: &str, body: &str, cfg: &Smells) -> Vec<SmellFinding> {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join(format!("{name}.py"));
    fs::write(&path, body).unwrap();
    let pf = parse_file(&path, 0).unwrap();
    detect_local(&pf, cfg)
}

fn find(f: &[SmellFinding], kind: &str) -> Option<SmellFinding> {
    f.iter().find(|s| s.kind.as_str() == kind).cloned()
}

#[test]
fn loose_typing_flags_dict_any_param_and_return() {
    let cfg = Smells::default();
    let body = "def f(payload: dict[str, Any], names: list[str]) -> dict:\n    return payload\n";
    let s = find(&local("lt", body, &cfg), "loose_typing").expect("must flag");
    assert_eq!(s.severity, Severity::Warning, "Any present → Warning");
    assert_eq!(s.metric, 3, "two params + return");
    assert!(s.message.contains("payload") && s.message.contains("names"));
}

#[test]
fn loose_typing_skips_domain_types_and_unannotated() {
    let cfg = Smells::default();
    let body = "def f(req: CreateUserRequest, x, n: int) -> UserModel:\n    return n\n";
    assert!(find(&local("lt_ok", body, &cfg), "loose_typing").is_none());
    // Containers OF domain types and Literal annotations are typed, not loose.
    let typed = "def g(refs: list[CustomFileRef], mode: Literal[\"json\", \"xml\"]) -> dict[str, UserModel]:\n    return {}\n";
    assert!(find(&local("lt_dom", typed, &cfg), "loose_typing").is_none());
}

#[test]
fn loose_typing_handles_optional_and_union() {
    let cfg = Smells::default();
    let body = "def f(a: Optional[dict[str, str]], b: list[int] | None):\n    return a\n";
    let s = find(&local("lt_opt", body, &cfg), "loose_typing").expect("must flag");
    assert_eq!(s.severity, Severity::Info, "no Any → Info");
    assert_eq!(s.metric, 2);
}

#[test]
fn magic_string_default_flags_empty_and_short_fallbacks() {
    let cfg = Smells::default();
    let body = "def f(name=None, title=None, code=None):\n    a = name or \"\"\n    b = title or \"?\"\n    c = code or \"ok\"\n    return a, b, c\n";
    let s = find(&local("msd", body, &cfg), "magic_string_default").expect("must flag");
    assert_eq!(s.severity, Severity::Warning);
    assert_eq!(s.metric, 2, "empty and one-char fallbacks count");
    assert!(s.message.contains("fallback string literal"));
    let ok = "def g(name=None):\n    return name or \"valid\"\n";
    assert!(find(&local("msd_ok", ok, &cfg), "magic_string_default").is_none());
}

#[test]
fn magic_string_default_flags_conditional_fallbacks_too() {
    let cfg = Smells::default();
    let body = "def f(name=None):\n    return name if name is not None else \"?\"\n";
    let s = find(&local("msd_if", body, &cfg), "magic_string_default").expect("must flag");
    assert_eq!(s.metric, 1);
}

#[test]
fn boolean_blindness_over_threshold_only() {
    let cfg = Smells::default(); // max_bool_params = 2
    let two = "def f(a: bool, b: bool, x: str):\n    return x\n";
    assert!(find(&local("bb2", two, &cfg), "boolean_blindness").is_none());
    let three = "def f(a: bool, b: bool, c: bool):\n    return a\n";
    let s = find(&local("bb3", three, &cfg), "boolean_blindness").expect("3 bools must flag");
    assert_eq!(s.metric, 3);
}

#[test]
fn tuple_packing_flags_bare_and_annotated_returns() {
    let cfg = Smells::default(); // max_tuple_return = 2
    let bare = "def f(x):\n    return x, x + 1, x + 2\n";
    let s = find(&local("tp_bare", bare, &cfg), "tuple_packing").expect("bare tuple");
    assert_eq!(s.metric, 3);
    let ann = "def g(x) -> tuple[str, int, dict[str, str]]:\n    return make(x)\n";
    let s = find(&local("tp_ann", ann, &cfg), "tuple_packing").expect("annotated tuple");
    assert_eq!(s.metric, 3, "bracket-aware arity");
    let pair = "def h(x):\n    return x, x + 1\n";
    assert!(find(&local("tp_pair", pair, &cfg), "tuple_packing").is_none());
}

#[test]
fn mutated_parameter_detects_subscript_method_and_del() {
    let cfg = Smells::default();
    let body = "def f(d, items, extra):\n    d[\"k\"] = 1\n    items.append(2)\n    del extra[\"x\"]\n    return d\n";
    let s = find(&local("mp", body, &cfg), "mutated_parameter").expect("must flag");
    assert_eq!(s.metric, 3);
    for p in ["d", "items", "extra"] {
        assert!(s.message.contains(p), "missing {p}: {}", s.message);
    }
}

#[test]
fn mutating_locals_or_self_is_fine() {
    let cfg = Smells::default();
    let body = "def f(self, x):\n    out = []\n    out.append(x)\n    self.cache.update(x)\n    return out\n";
    assert!(find(&local("mp_ok", body, &cfg), "mutated_parameter").is_none());
}

#[test]
fn param_attr_mutation_is_opt_in() {
    // Mutating a parameter's attribute contents (`msg.kwargs[k]=v`,
    // `msg.items.append(...)`) is caller-visible but only flagged when the
    // stricter `param_attr_mutation` flag is on.
    let body = "def f(msg):\n    msg.additional_kwargs[\"k\"] = 1\n    msg.items.append(2)\n    return msg\n";

    let off = Smells::default(); // param_attr_mutation = false
    assert!(
        find(&local("pam_off", body, &off), "mutated_parameter").is_none(),
        "attribute-deep mutation must NOT flag by default"
    );

    let on = Smells {
        param_attr_mutation: true,
        ..Smells::default()
    };
    let s = find(&local("pam_on", body, &on), "mutated_parameter")
        .expect("attribute-deep mutation must flag when opted in");
    assert!(s.message.contains("msg"), "names the param: {}", s.message);
}

#[test]
fn param_attr_mutation_still_spares_self_and_locals() {
    // Even with the strict flag on, `self.cache.update()` (own state) and a
    // local's attribute mutation must not flag — only parameters do.
    let on = Smells {
        param_attr_mutation: true,
        ..Smells::default()
    };
    let body = "def f(self, x):\n    out = {}\n    out.data.append(x)\n    self.cache.update(x)\n    return out\n";
    assert!(find(&local("pam_self", body, &on), "mutated_parameter").is_none());
}

#[test]
fn reassigned_parameter_is_opt_in() {
    let body = "def f(x=None):\n    x = x or []\n    return x\n";
    let off = Smells::default();
    assert!(find(&local("rp_off", body, &off), "reassigned_parameter").is_none());
    let on = Smells {
        param_reassignment: true,
        ..Smells::default()
    };
    assert!(find(&local("rp_on", body, &on), "reassigned_parameter").is_some());
}

#[test]
fn implicit_schema_needs_distinct_keys_and_dictish_type() {
    let cfg = Smells::default(); // implicit_schema_min_keys = 4
    let body = "def f(cfg):\n    a = cfg[\"host\"]\n    b = cfg[\"port\"]\n    c = cfg[\"user\"]\n    d = cfg[\"pass\"]\n    return a, b\n";
    let s = find(&local("is4", body, &cfg), "implicit_schema").expect("4 keys must flag");
    assert_eq!(s.metric, 4);
    // Same keys but receiver annotated as a DataFrame → typed, skipped.
    let typed = "def f(cfg: DataFrame):\n    a = cfg[\"host\"]\n    b = cfg[\"port\"]\n    c = cfg[\"user\"]\n    d = cfg[\"pass\"]\n    return a\n";
    assert!(find(&local("is_df", typed, &cfg), "implicit_schema").is_none());
    // Repeated access to ONE key is not a schema.
    let one =
        "def f(cfg):\n    return cfg[\"host\"] + cfg[\"host\"] + cfg[\"host\"] + cfg[\"host\"]\n";
    assert!(find(&local("is1", one, &cfg), "implicit_schema").is_none());
}

#[test]
fn literal_membership_flags_string_lists_only() {
    let cfg = Smells::default();
    let body = "def f(role):\n    if role in [\"admin\", \"super_user\"]:\n        return True\n    return False\n";
    let s = find(&local("lm", body, &cfg), "literal_membership").expect("must flag");
    assert_eq!(s.metric, 1);
    // Membership in a variable or non-string list is fine.
    let ok = "def f(x, allowed):\n    return x in allowed or x in [1, 2, 3]\n";
    assert!(find(&local("lm_ok", ok, &cfg), "literal_membership").is_none());
}

#[test]
fn heavy_nested_function_flagged_but_thin_wrappers_pass() {
    let cfg = Smells {
        max_nested_function_lines: 5,
        ..Smells::default()
    };
    // A nested def with real logic must flag, attributed to its parent.
    let heavy = format!(
        "def outer(items):\n    def process(batch):\n{}        return batch\n    return process(items)\n",
        "        batch = transform(batch)\n".repeat(8)
    );
    let s = find(&local("hn", &heavy, &cfg), "heavy_nested_function").expect("must flag");
    assert_eq!(s.symbol, "process");
    assert!(s.message.contains("`outer`"), "{}", s.message);
    assert!(s.message.contains("unit-tested"), "{}", s.message);
    // A thin closure stays silent; so does a method on a class (not nested).
    let thin = "def outer(x):\n    def add(y):\n        return x + y\n    return add\n";
    assert!(find(&local("hn_thin", thin, &cfg), "heavy_nested_function").is_none());
    let method = format!(
        "class C:\n    def m(self):\n{}        return 1\n",
        "        x = 1\n".repeat(8)
    );
    assert!(find(&local("hn_method", &method, &cfg), "heavy_nested_function").is_none());
    // 0 disables the detector entirely.
    let off = Smells {
        max_nested_function_lines: 0,
        ..Smells::default()
    };
    assert!(find(&local("hn_off", &heavy, &off), "heavy_nested_function").is_none());
}

#[test]
fn discipline_toggles_disable_detectors() {
    let cfg = Smells {
        loose_typing: false,
        magic_string_default: false,
        tuple_packing: false,
        param_mutation: false,
        literal_membership: false,
        implicit_schema_min_keys: 0,
        ..Smells::default()
    };
    let body = "def f(d: dict, role):\n    d[\"k\"] = 1\n    if role in [\"a\", \"b\"]:\n        return d, role, 1\n    return None, None, 0\n";
    assert!(
        local("off", body, &cfg).is_empty(),
        "all discipline detectors disabled must be silent"
    );
}
