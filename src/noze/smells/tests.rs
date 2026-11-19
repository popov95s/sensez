use super::test_support as fixture;
use super::*;
use crate::config::smells::Smells;
use crate::report::SmellFinding;

/// Write `body` to a temp `.py` file and parse it.
fn parsed(name: &str, body: &str) -> crate::spine::parser::ParsedFile {
    fixture::parsed(&format!("{name}.py"), body)
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
    let deep = "def f(invoice):\n    return invoice.customer.profile.address.country\n";
    assert!(has(&local("mc_deep", deep, &cfg), "message_chain"));
    let shallow = "def f(invoice):\n    return invoice.total()\n";
    assert!(!has(&local("mc_shallow", shallow, &cfg), "message_chain"));
    let builder = "def q(db):\n    return db.query(Model).filter(a).order_by(b).limit(10).all()\n";
    assert!(
        !has(&local("mc_builder", builder, &cfg), "message_chain"),
        "fluent builder chains must not trip the Demeter check"
    );
}

#[test]
fn feature_envy_only_methods_with_own_state() {
    let cfg = Smells::default();
    let free_fn =
        "def handle(payload: Request):\n    return payload.a + payload.b + payload.c + payload.d\n";
    assert!(
        !has(&local("fe_free", free_fn, &cfg), "feature_envy"),
        "free functions must never be feature envy"
    );

    let envious = "class Calc:\n    def run(self, order: Order):\n        self.cache\n        return order.a + order.b + order.c + order.d\n";
    assert!(
        has(&local("fe_method", envious, &cfg), "feature_envy"),
        "a method neglecting self for another typed object is envy"
    );

    let cohesive = "class Calc:\n    def run(self, order: Order):\n        self.a\n        self.b\n        self.c\n        self.d\n        return order.x\n";
    assert!(
        !has(&local("fe_cohesive", cohesive, &cfg), "feature_envy"),
        "self-heavy method is not envy"
    );

    let untyped = "class Calc:\n    def run(self, order):\n        self.cache\n        return order.a + order.b + order.c + order.d\n";
    assert!(
        !has(&local("fe_untyped", untyped, &cfg), "feature_envy"),
        "unknown receiver type must skip (precision over recall)"
    );
}

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
    let body = "class Base:\n    pass\n\nclass Sub(Base):\n    def a(self):\n        raise NotImplementedError\n    def b(self):\n        pass\n";
    assert!(has(&local("rb", body, &cfg), "refused_bequest"));
}

#[test]
fn abstract_base_is_not_refused_bequest() {
    let cfg = Smells::default();
    let abc = "from abc import ABC, abstractmethod\n\nclass Strategy(ABC):\n    @abstractmethod\n    def run(self):\n        pass\n    @abstractmethod\n    def name(self):\n        pass\n    @abstractmethod\n    def reset(self):\n        pass\n";
    assert!(
        !has(&local("rb_abc", abc, &cfg), "refused_bequest"),
        "an abstract base class is not a refused bequest"
    );
}

#[test]
fn divergent_change_flagged_for_incohesive_class() {
    let cfg = Smells::default();
    let body = "class Mixed:\n    def load_user(self):\n        return self.user_db\n    def save_user(self):\n        self.user_db.commit()\n    def render_html(self):\n        return self.template\n    def render_css(self):\n        return self.template.css\n";
    let f = local("dc_bad", body, &cfg);
    assert!(has(&f, "divergent_change"), "kinds: {:?}", kinds(&f));
}

#[test]
fn divergent_change_skips_stateless_and_data_classes() {
    let cfg = Smells::default();
    let crud = "class CRUDThing:\n    @staticmethod\n    def get(db, id):\n        return db.get(id)\n    @staticmethod\n    def create(db, data):\n        return db.add(data)\n    @staticmethod\n    def delete(db, id):\n        return db.remove(id)\n    @staticmethod\n    def list(db):\n        return db.all()\n";
    assert!(
        !has(&local("dc_crud", crud, &cfg), "divergent_change"),
        "stateless CRUD repo must not be flagged"
    );
    let settings = "class Settings(BaseSettings):\n    NAME: str = \"x\"\n    PORT: int = 8000\n    DEBUG: bool = False\n    def url(self):\n        return self.NAME\n";
    assert!(
        !has(&local("dc_settings", settings, &cfg), "divergent_change"),
        "data/settings classes must not be flagged"
    );
}
