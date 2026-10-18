//! Smell message wording for TypeScript.

use super::tests::findings_for;
use crate::config::smells::Smells;
use crate::report::SmellKind;

#[test]
fn ts_messages_carry_no_python_idioms() {
    let src = b"export function handle(cfg: Record<string, any>, a: boolean, b: boolean, c: boolean): [string, number, boolean] {\n  if ([\"x\", \"y\"].includes(cfg.kind)) { return [\"x\", 7, true]; }\n  return [\"y\", 8, false];\n}\nexport async function link(p?: { id: string }) { await save({ parentId: p?.id ?? \"\" }); }\nexport function pump(items: any[]): void { items.push(1); }\n";
    let cfg = Smells {
        disabled: Vec::new(),
        ..Smells::default()
    };
    let findings = findings_for(src, &cfg);
    for kind in [
        SmellKind::LooseTyping,
        SmellKind::BooleanBlindness,
        SmellKind::TuplePacking,
        SmellKind::LiteralMembership,
        SmellKind::MagicStringDefault,
    ] {
        assert!(
            findings.iter().any(|f| f.kind == kind),
            "{kind:?}: {findings:?}"
        );
    }
    for finding in &findings {
        for idiom in [
            "`or ",
            "Enum",
            "NamedTuple",
            "dataclass",
            "True",
            "False",
            "keyword args",
        ] {
            assert!(
                !finding.message.contains(idiom),
                "{idiom} in {:?}: {}",
                finding.kind,
                finding.message
            );
        }
    }
}
