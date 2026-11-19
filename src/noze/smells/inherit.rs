use super::{make, Measure, SmellContext};
use crate::config::smells::Smells;
use crate::report::{Severity, SmellFinding, SmellKind};
use crate::spine::ir::ClassUnit;

pub fn detect(
    ctx: &SmellContext<'_>,
    classes: &[ClassUnit],
    _cfg: &Smells,
    out: &mut Vec<SmellFinding>,
) {
    for class in classes {
        if class.bases.is_empty() || class.is_abstract {
            continue;
        }
        let stubs = class.overrides_to_stub.len();
        let methods = class.methods.len().max(1);
        if stubs >= 2 && stubs * 2 >= methods {
            out.push(make(
                SmellKind::RefusedBequest,
                format!(
                    "{stubs} of {methods} methods are stubs (pass/NotImplementedError) \
                     despite inheriting {} — refused bequest",
                    class.bases.join(", ")
                ),
                ctx.path,
                class.start_line,
                &class.name,
                Severity::Warning,
                Measure::new(stubs, methods),
            ));
        }
    }
}
