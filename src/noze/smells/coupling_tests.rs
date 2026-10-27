use super::detect_local;
use super::test_support as fixture;
use crate::config::smells::Smells;
use crate::report::{SmellFinding, SmellKind};

fn envy_ts(source: &str) -> Vec<SmellFinding> {
    envy("sample.ts", source)
}

fn envy(name: &str, source: &str) -> Vec<SmellFinding> {
    let file = fixture::parsed(name, source);
    detect_local(&file, &Smells::default())
        .into_iter()
        .filter(|finding| finding.kind == SmellKind::FeatureEnvy)
        .collect()
}

#[test]
fn inline_types_constructors_and_ingest_methods_are_not_envy() {
    for source in [
        "export class Box {\n  a = 0;\n  apply(opts: {\n    a: number;\n    b: number;\n  }) {\n    return this.a + opts.a + opts.b + opts.a + opts.b;\n  }\n}\n",
        "export class Box {\n  load(o: Opts) {\n    this.b = o.a + o.b + o.c + o.d;\n  }\n}\n",
        "export class Box {\n  constructor(o: Opts) {\n    this.total = this.seed + o.a + o.b + o.c + o.d + o.e;\n  }\n}\n",
    ] {
        let findings = envy_ts(source);
        assert!(findings.is_empty(), "{source}\n{findings:?}");
    }
}

#[test]
fn method_reading_a_nominal_foreign_type_is_envy() {
    let findings = envy_ts(
        "export class Billing {\n  rate = 1;\n  total(o: Order) {\n    return this.rate * (o.net + o.tax + o.fee + o.ship);\n  }\n}\n",
    );
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert!(
        findings[0].message.contains("(Order)"),
        "{}",
        findings[0].message
    );
}

#[test]
fn nullable_nominal_types_are_still_envy() {
    let python = "class Billing:\n    def total(self, o: Order | None):\n        return self.rate * (o.net + o.tax + o.fee + o.ship)\n";
    assert_eq!(envy("sample.py", python).len(), 1);
    let optional = python.replace("Order | None", "Optional[Order]");
    assert_eq!(envy("sample.py", &optional).len(), 1);
    let ts = "export class Billing {\n  rate = 1;\n  total(o: Order | undefined) {\n    return this.rate * (o.net + o.tax + o.fee + o.ship);\n  }\n}\n";
    assert_eq!(envy_ts(ts).len(), 1);
}
