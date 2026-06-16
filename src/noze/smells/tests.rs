use super::*;
use crate::config::smells::Smells;
use crate::spine::parser::parse_file;
use std::fs;
use std::path::{Path, PathBuf};

/// Write `body` to a temp `.py` file and parse it.
fn parsed(name: &str, body: &str) -> crate::spine::parser::ParsedFile {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join(format!("{name}.py"));
    fs::write(&path, body).unwrap();
    parse_file(&path, 0).unwrap()
}

fn local(name: &str, body: &str, cfg: &Smells) -> Vec<SmellFinding> {
    let pf = parsed(name, body);
    detect_local(&pf, cfg)
}

fn kinds(findings: &[SmellFinding]) -> Vec<&str> {
    findings.iter().map(|f| f.kind.as_str()).collect()
}

fn has(findings: &[SmellFinding], kind: &str) -> bool {
    findings.iter().any(|f| f.kind.as_str() == kind)
}

#[test]
fn clean_code_is_silent() {
    let cfg = Smells::default();
    let body = "def add(a, b):\n    return a + b\n";
    assert!(
        local("clean", body, &cfg).is_empty(),
        "clean code must yield no smells"
    );
}

#[test]
fn high_complexity_flagged() {
    let cfg = Smells {
        cyclomatic_complexity: true,
        ..Smells::default()
    };
    let mut body = String::from("def f(x):\n");
    for i in 0..12 {
        body.push_str(&format!("    if x == {i} or x > {i}:\n        x += 1\n"));
    }
    body.push_str("    return x\n");
    let f = local("cx", &body, &cfg);
    assert!(has(&f, "high_complexity"), "kinds: {:?}", kinds(&f));
}

#[test]
fn long_function_flagged() {
    let cfg = Smells {
        long_function: true,
        max_function_lines: 5,
        ..Smells::default()
    };
    let body = format!("def f():\n{}", "    x = 1\n".repeat(10));
    assert!(has(&local("lf", &body, &cfg), "long_function"));
}

#[test]
fn deep_nesting_flagged() {
    let cfg = Smells {
        max_nesting: 2,
        ..Smells::default()
    };
    let body = "def f(x):\n    if x:\n        for i in x:\n            while i:\n                return i\n";
    assert!(has(&local("dn", body, &cfg), "deep_nesting"));
}

#[test]
fn long_parameter_list_excludes_self() {
    let cfg = Smells {
        long_parameter_list: true,
        max_params: 3,
        ..Smells::default()
    };
    // 3 real params + self -> not flagged (self excluded).
    let ok = "class C:\n    def m(self, a, b, c):\n        return a\n";
    assert!(!has(&local("lp_ok", ok, &cfg), "long_parameter_list"));
    let bad = "def f(a, b, c, d):\n    return a\n";
    assert!(has(&local("lp_bad", bad, &cfg), "long_parameter_list"));
}

#[test]
fn message_chain_positive_and_negative() {
    let cfg = Smells {
        max_chain_depth: 3,
        ..Smells::default()
    };
    // Pure attribute navigation is a Law-of-Demeter violation.
    let deep = "def f(invoice):\n    return invoice.customer.profile.address.country\n";
    assert!(has(&local("mc_deep", deep, &cfg), "message_chain"));
    let shallow = "def f(invoice):\n    return invoice.total()\n";
    assert!(!has(&local("mc_shallow", shallow, &cfg), "message_chain"));
    // A fluent/builder call chain is idiomatic, NOT a message chain — the chain
    // runs through call results, not consecutive attribute accesses.
    let builder = "def q(db):\n    return db.query(Model).filter(a).order_by(b).limit(10).all()\n";
    assert!(
        !has(&local("mc_builder", builder, &cfg), "message_chain"),
        "fluent builder chains must not trip the Demeter check"
    );
}

#[test]
fn feature_envy_only_methods_with_own_state() {
    let cfg = Smells::default();
    // A free function (e.g. an HTTP endpoint) reading a typed argument heavily is
    // NOT feature envy — there is no owning object whose behavior it neglects.
    let free_fn =
        "def handle(payload: Request):\n    return payload.a + payload.b + payload.c + payload.d\n";
    assert!(
        !has(&local("fe_free", free_fn, &cfg), "feature_envy"),
        "free functions must never be feature envy"
    );

    // A method that ignores its own state and works another object > 2x as much.
    let envious = "class Calc:\n    def run(self, order: Order):\n        self.cache\n        return order.a + order.b + order.c + order.d\n";
    assert!(
        has(&local("fe_method", envious, &cfg), "feature_envy"),
        "a method neglecting self for another typed object is envy"
    );

    // A method that mostly uses its own state is fine.
    let cohesive = "class Calc:\n    def run(self, order: Order):\n        self.a\n        self.b\n        self.c\n        self.d\n        return order.x\n";
    assert!(
        !has(&local("fe_cohesive", cohesive, &cfg), "feature_envy"),
        "self-heavy method is not envy"
    );

    // Even an envious-shaped method is skipped when the receiver type is unknown.
    let untyped = "class Calc:\n    def run(self, order):\n        self.cache\n        return order.a + order.b + order.c + order.d\n";
    assert!(
        !has(&local("fe_untyped", untyped, &cfg), "feature_envy"),
        "unknown receiver type must skip (precision over recall)"
    );
}

/// Regression: when two non-self receivers tie on access count and only one
/// resolves to a type, the pick (and therefore the finding) must be stable. A
/// bare `max_by_key` over the `receiver_access` HashMap returned whichever tied
/// receiver iteration yielded last — and that order is randomized per map — so
/// the finding flickered on/off run to run. Each `local()` re-parses, building a
/// fresh map with a fresh seed, so a non-deterministic tie-break flakes here.
/// The class LCOM map (`method_attr_use`) is now assembled from each method's
/// own `FunctionUnit.self_attrs` (collected during its single body walk). It
/// must capture the method's own `self.<attr>` and exclude accesses inside
/// nested functions.
#[test]
fn method_attr_use_captures_own_self_attrs_excluding_nested() {
    let src = "class C:\n    def m(self):\n        self.a\n        self.b\n        def inner():\n            self.c\n";
    let pf = parsed("mau", src);
    let class = pf
        .walked
        .units
        .classes
        .iter()
        .find(|c| c.name == "C")
        .unwrap();
    let attrs = class
        .method_attr_use
        .get("m")
        .expect("method m must be recorded in the LCOM map");
    assert!(
        attrs.contains("a") && attrs.contains("b"),
        "own-body self.<attr> must be captured"
    );
    assert!(
        !attrs.contains("c"),
        "self.<attr> inside a nested function belongs to that scope, not the method"
    );
}

#[test]
fn feature_envy_is_deterministic_on_ties() {
    let cfg = Smells::default();
    // `order` (typed) and `blob` (untyped) are each touched 3×; `self` once — a
    // dead-even tie whose two candidates disagree on whether their type is known.
    let tied = "class Calc:\n    def run(self, order: Order, blob):\n        self.cache\n        order.a\n        order.b\n        order.c\n        blob.x\n        blob.y\n        blob.z\n";
    let baseline = has(&local("fe_tie", tied, &cfg), "feature_envy");
    for _ in 0..50 {
        assert_eq!(
            has(&local("fe_tie", tied, &cfg), "feature_envy"),
            baseline,
            "feature_envy must be deterministic across runs on a receiver-count tie"
        );
    }
}

#[test]
fn refused_bequest_flagged() {
    let cfg = Smells::default();
    // A concrete subclass stubbing out most inherited (concrete) behavior.
    let body = "class Base:\n    pass\n\nclass Sub(Base):\n    def a(self):\n        raise NotImplementedError\n    def b(self):\n        pass\n";
    assert!(has(&local("rb", body, &cfg), "refused_bequest"));
}

#[test]
fn abstract_base_is_not_refused_bequest() {
    let cfg = Smells::default();
    // An ABC declaring abstract methods (bodies are `...`/`pass`) is correct —
    // it must not be reported as refused bequest.
    let abc = "from abc import ABC, abstractmethod\n\nclass Strategy(ABC):\n    @abstractmethod\n    def run(self):\n        pass\n    @abstractmethod\n    def name(self):\n        pass\n    @abstractmethod\n    def reset(self):\n        pass\n";
    assert!(
        !has(&local("rb_abc", abc, &cfg), "refused_bequest"),
        "an abstract base class is not a refused bequest"
    );
}

#[test]
fn divergent_change_flagged_for_incohesive_class() {
    let cfg = Smells::default();
    // Four stateful methods forming two disjoint attribute clusters, no __init__
    // bridging them -> low cohesion.
    let body = "class Mixed:\n    def load_user(self):\n        return self.user_db\n    def save_user(self):\n        self.user_db.commit()\n    def render_html(self):\n        return self.template\n    def render_css(self):\n        return self.template.css\n";
    let f = local("dc_bad", body, &cfg);
    assert!(has(&f, "divergent_change"), "kinds: {:?}", kinds(&f));
}

#[test]
fn divergent_change_skips_stateless_and_data_classes() {
    let cfg = Smells::default();
    // CRUD-style repo: @staticmethods take `db`, never touch self -> no signal.
    let crud = "class CRUDThing:\n    @staticmethod\n    def get(db, id):\n        return db.get(id)\n    @staticmethod\n    def create(db, data):\n        return db.add(data)\n    @staticmethod\n    def delete(db, id):\n        return db.remove(id)\n    @staticmethod\n    def list(db):\n        return db.all()\n";
    assert!(
        !has(&local("dc_crud", crud, &cfg), "divergent_change"),
        "stateless CRUD repo must not be flagged"
    );
    // Pydantic-style settings: fields, not behavior.
    let settings = "class Settings(BaseSettings):\n    NAME: str = \"x\"\n    PORT: int = 8000\n    DEBUG: bool = False\n    def url(self):\n        return self.NAME\n";
    assert!(
        !has(&local("dc_settings", settings, &cfg), "divergent_change"),
        "data/settings classes must not be flagged"
    );
}

#[test]
fn god_module_is_a_hub_not_an_entrypoint() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    write(&dir, "pkg/__init__.py", "");
    // hub imports 4 leaves AND is imported by 4 dependents -> a real hub.
    write(
        &dir,
        "pkg/hub.py",
        "from pkg.l0 import a\nfrom pkg.l1 import a\nfrom pkg.l2 import a\nfrom pkg.l3 import a\n",
    );
    for n in 0..4 {
        write(&dir, &format!("pkg/l{n}.py"), "a = 1\n");
        write(&dir, &format!("pkg/d{n}.py"), "from pkg.hub import x\n");
    }
    // entry imports 8 modules but nobody imports it -> a composition root.
    write(
        &dir,
        "pkg/entry.py",
        "from pkg.l0 import a\nfrom pkg.l1 import a\nfrom pkg.l2 import a\nfrom pkg.l3 import a\nfrom pkg.d0 import x\nfrom pkg.d1 import x\nfrom pkg.d2 import x\nfrom pkg.d3 import x\n",
    );

    let paths = [
        "pkg/hub.py",
        "pkg/l0.py",
        "pkg/l1.py",
        "pkg/l2.py",
        "pkg/l3.py",
        "pkg/d0.py",
        "pkg/d1.py",
        "pkg/d2.py",
        "pkg/d3.py",
        "pkg/entry.py",
        "pkg/__init__.py",
    ];
    let parsed: Vec<_> = paths
        .iter()
        .enumerate()
        .map(|(i, p)| parse_file(&dir.join(p), i as u32).unwrap())
        .collect();
    let graph = crate::spine::graph::build(&parsed, &[]);
    // Raise the shotgun threshold so the hub is evaluated as a god module rather
    // than being claimed first by the shotgun-hazard check.
    let cfg = Smells {
        god_module_fan: 8,
        shotgun_blast_threshold: 100,
        ..Smells::default()
    };
    let gm: Vec<String> = graphy::detect(&graph, &cfg.clone().into())
        .into_iter()
        .filter(|s| s.kind == SmellKind::GodModule)
        .map(|s| s.symbol)
        .collect();
    assert!(
        gm.iter().any(|m| m.ends_with("hub")),
        "the hub must be flagged: {gm:?}"
    );
    assert!(
        !gm.iter().any(|m| m.ends_with("entry")),
        "a pure entrypoint (fan-in 0) must not be a god module: {gm:?}"
    );
}

#[test]
fn primitive_params_alone_are_not_a_smell() {
    // Plain str/int/float params are idiomatic and must stay silent.
    let cfg = Smells::default();
    let body = "def f(a: str, b: int, c: float):\n    return a\n";
    assert!(local("po_off", body, &cfg).is_empty());
}

#[test]
fn linter_owned_smells_are_off_by_default() {
    let cfg = Smells::default();
    // Many params, magic number, 6 returns — linter-owned, so silent here.
    let body = "def f(a, b, c, d, e, f, g):\n    x = 9999\n    if a: return 1\n    if b: return 2\n    if c: return 3\n    if d: return 4\n    if e: return 5\n    return x\n";
    let found = local("linter_off", body, &cfg);
    for kind in [
        "magic_numbers",
        "long_parameter_list",
        "too_many_returns",
        "high_complexity",
    ] {
        assert!(
            !has(&found, kind),
            "{kind} should be off by default (language linters cover it)"
        );
    }
}

#[test]
fn data_clumps_need_min_support() {
    let cfg = Smells::default(); // min_fields = 4, min_occurrences = 3
    let sig = "(start, end, tz, fmt)";
    let two = format!("def a{sig}:\n    return 1\ndef b{sig}:\n    return 2\n");
    let pf = parsed("clump2", &two);
    assert!(
        !has(&clumps::detect(&[&pf], &cfg.clone().into()), "data_clump"),
        "2 < 3 occurrences"
    );
    let three = format!("{two}def c{sig}:\n    return 3\n");
    let pf3 = parsed("clump3", &three);
    assert!(has(
        &clumps::detect(&[&pf3], &cfg.clone().into()),
        "data_clump"
    ));
}

#[test]
fn data_clump_respects_min_fields_default() {
    let cfg = Smells::default(); // min_fields = 4
                                 // A recurring 3-field bundle must NOT trigger at the default minimum of 4.
    let body = "def a(x, y, z):\n    return 1\ndef b(x, y, z):\n    return 2\ndef c(x, y, z):\n    return 3\n";
    let pf = parsed("clump_3field", body);
    assert!(
        !has(&clumps::detect(&[&pf], &cfg.clone().into()), "data_clump"),
        "3-field bundle is below the default min_fields of 4"
    );
}

fn write(dir: &Path, rel: &str, body: &str) -> PathBuf {
    let path = dir.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, body).unwrap();
    path
}

#[test]
fn shotgun_hazard_needs_blast_radius() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    let core = write(&dir, "pkg/core.py", "def helper():\n    return 1\n");
    let mut files = vec![core];
    // Five distinct dependents import pkg.core -> blast = 5 >= threshold 4.
    for i in 0..5 {
        files.push(write(
            &dir,
            &format!("pkg/m{i}.py"),
            "from pkg.core import helper\n",
        ));
    }
    write(&dir, "pkg/__init__.py", "");
    let parsed: Vec<_> = files
        .iter()
        .enumerate()
        .map(|(i, p)| parse_file(p, i as u32).unwrap())
        .collect();
    let graph = crate::spine::graph::build(&parsed, &[]);
    let cfg = Smells::default();
    let findings = graphy::detect(&graph, &cfg.clone().into());
    assert!(
        has(&findings, "shotgun_surgery_hazard"),
        "kinds: {:?}",
        kinds(&findings)
    );
}

#[test]
fn package_index_barrels_are_not_graph_hotspots() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    let barrel = write(&dir, "pkg/__init__.py", "from pkg.core import helper\n");
    write(&dir, "pkg/core.py", "def helper():\n    return 1\n");
    let mut files = vec![barrel];
    for i in 0..5 {
        files.push(write(&dir, &format!("pkg/m{i}.py"), "import pkg\n"));
    }
    let parsed: Vec<_> = files
        .iter()
        .enumerate()
        .map(|(i, p)| parse_file(p, i as u32).unwrap())
        .collect();
    let graph = crate::spine::graph::build(&parsed, &[]);
    let cfg = Smells::default();
    let findings = graphy::detect(&graph, &cfg.clone().into());
    assert!(
        findings
            .iter()
            .all(|f| !f.file.ends_with("pkg/__init__.py")),
        "package API barrels should not be graph smell hotspots: {findings:?}"
    );
}

#[test]
fn split_variable_flags_branch_bound_local_at_min_two() {
    // The motivating case: one name bound in both arms of an if/else. At
    // min_assigns = 2 this enforces single-binding locals.
    let cfg = Smells {
        split_variable: true,
        split_variable_min_assigns: 2,
        ..Smells::default()
    };
    let body = "def f(x):\n    if x:\n        plan = make(x)\n    else:\n        plan = load()\n    return plan\n";
    let f = local("sv_branch", body, &cfg);
    assert!(has(&f, "split_variable"), "kinds: {:?}", kinds(&f));
}

#[test]
fn split_variable_default_min_flags_double_assignment() {
    // Default min_assigns is 2: enabling the smell catches branch-bound locals.
    let cfg = Smells {
        split_variable: true,
        ..Smells::default()
    };
    let body = "def f(x):\n    if x:\n        plan = make(x)\n    else:\n        plan = load()\n    return plan\n";
    assert!(has(&local("sv_default", body, &cfg), "split_variable"));
}

#[test]
fn split_variable_single_binding_is_silent() {
    let cfg = Smells {
        split_variable: true,
        split_variable_min_assigns: 2,
        ..Smells::default()
    };
    let body = "def f(x):\n    plan = make(x)\n    return plan\n";
    assert!(!has(&local("sv_single", body, &cfg), "split_variable"));
}

#[test]
fn disabled_pillar_is_silent() {
    let cfg = Smells {
        enabled: false,
        ..Smells::default()
    };
    let pf = parsed("disabled", "def f(a,b,c,d,e,f,g):\n    return 1\n");
    assert!(detect(&[pf], &CodebaseGraph::default(), &cfg.clone().into()).is_empty());
}

use crate::spine::graph::CodebaseGraph;
