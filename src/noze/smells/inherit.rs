//! Refused Bequest: a subclass that stubs out most of what it inherits.
//!
//! Detected within-file from the class's own base list plus the count of
//! methods whose body is only `pass`/`raise NotImplementedError`. Cross-file
//! base-method comparison is a future refinement (the import graph carries the
//! edge); this conservative form flags clear stub-heavy overrides.

use super::{make, SmellContext};
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
        // No bases, or the class is itself abstract (an ABC declaring abstract
        // methods is correct, not a refused bequest).
        if class.bases.is_empty() || class.is_abstract {
            continue;
        }
        let stubs = class.overrides_to_stub.len();
        let methods = class.methods.len().max(1);
        // Flag when stubbing is substantial: 2+ stubs and at least half of methods.
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
                stubs as u32,
                methods as u32,
            ));
        }
    }
}
