//! Test files that replace the module their subject under test comes from.

use super::mock_modules::{ModuleKeys, ReachedMember};
use super::{make, SmellContext};
use crate::config::smells::{Smells, Strictness};
use crate::report::{Severity, SmellFinding, SmellKind};
use crate::spine::ir::{FunctionUnit, MockFidelity, MockHost, MockScope, MockTarget};
use crate::spine::parser::ParsedFile;

pub(super) fn detect(
    file: &ParsedFile,
    ctx: &SmellContext<'_>,
    config: &Smells,
) -> Vec<SmellFinding> {
    let mocks = &file.walked.units.mock_targets;
    if !ctx.test_source || mocks.is_empty() || !config.is_enabled(SmellKind::MockedSubject) {
        return Vec::new();
    }
    let calls = subject_calls(
        &file.walked.units.functions,
        ctx.wording.double_api_prefixes,
    );
    let mut keys = ModuleKeys::for_file(file);
    let reached: Vec<ReachedMember> = file
        .walked
        .symbols
        .imports
        .iter()
        .filter(|import| !import.is_inline)
        .flat_map(|import| keys.reached(import, &calls))
        .collect();
    let strictness = config.mocked_subject_strictness;
    mocks
        .iter()
        .filter(|mock| mock.fidelity == MockFidelity::Replaces)
        .filter_map(|mock| verdict(mock, &reached, &mut keys, ctx, strictness))
        .map(|(mock, subject)| finding(ctx, mock, &subject))
        .collect()
}

/// Call paths in test cases and the callbacks nested inside them.
fn subject_calls<'a>(units: &'a [FunctionUnit], double_api: &[&str]) -> Vec<&'a str> {
    let tests: Vec<_> = units
        .iter()
        .filter(|unit| unit.review_risks.is_test_case)
        .map(|unit| unit.start_line..=unit.end_line)
        .collect();
    units
        .iter()
        .filter(|unit| {
            tests
                .iter()
                .any(|span| span.contains(&unit.start_line) && span.contains(&unit.end_line))
        })
        .flat_map(|unit| &unit.performance.calls)
        .map(|call| call.target.as_str())
        .filter(|target| {
            !target
                .split('.')
                .any(|segment| is_double_api(segment, double_api))
        })
        .collect()
}

fn is_double_api(segment: &str, prefixes: &[&str]) -> bool {
    prefixes.iter().any(|prefix| segment.starts_with(prefix))
}

/// The mock and the subject it replaces, when the strictness level reports it.
fn verdict<'m>(
    mock: &'m MockTarget,
    reached: &[ReachedMember],
    keys: &mut ModuleKeys<'_>,
    ctx: &SmellContext<'_>,
    strictness: Strictness,
) -> Option<(&'m MockTarget, String)> {
    let high = strictness == Strictness::High;
    let subject = match &mock.host {
        MockHost::Specifier(specifier) => {
            let module = keys.specifier(specifier);
            reached
                .iter()
                .find(|subject| replaces(mock, &module, subject))
                .map(|subject| subject.binding.clone())
                .or_else(|| (high && shares_test_name(&module, ctx)).then(|| module.clone()))
        }
        MockHost::Binding(object) => reached
            .iter()
            .find(|subject| high && subject.binding == *object)
            .map(|subject| subject.binding.clone()),
    }?;
    Some((mock, subject))
}

fn replaces(mock: &MockTarget, module: &str, subject: &ReachedMember) -> bool {
    subject.module == module
        && match &mock.scope {
            MockScope::WholeModule => true,
            MockScope::Member(member) => subject.member.as_deref() == Some(member),
        }
}

/// The mocked module is named like the test file and sits in a directory on the test's path.
fn shares_test_name(module: &str, ctx: &SmellContext<'_>) -> bool {
    let mut segments = module.rsplit(['/', '.']);
    let named_alike = segments.next() == Some(test_stem(ctx));
    let near = segments.next().is_none_or(|parent| {
        ctx.path
            .components()
            .any(|component| component.as_os_str() == parent)
    });
    named_alike && near
}

/// The test file name without test affixes (`pricing.test.ts`, `test_pricing.py` -> `pricing`).
fn test_stem<'p>(ctx: &SmellContext<'p>) -> &'p str {
    let name = ctx
        .path
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.split('.').next())
        .unwrap_or_default();
    let unprefixed = name
        .strip_prefix(ctx.wording.test_file_prefix)
        .unwrap_or(name);
    unprefixed
        .strip_suffix(ctx.wording.test_file_suffix)
        .unwrap_or(unprefixed)
}

fn finding(ctx: &SmellContext<'_>, mock: &MockTarget, subject: &str) -> SmellFinding {
    let host = match &mock.host {
        MockHost::Specifier(specifier) | MockHost::Binding(specifier) => specifier,
    };
    make(
        SmellKind::MockedSubject,
        format!(
            "the test replaces `{host}`, where its subject `{subject}` comes from, with a test double; it exercises the double instead of the code"
        ),
        ctx.path,
        mock.line,
        subject,
        Severity::Warning,
        1,
        0,
    )
}
