//! Test assertions that cannot fail.

use super::review_risks::{calls_check, checked_helpers};
use super::{make, SmellContext};
use crate::config::smells::Smells;
use crate::report::{Severity, SmellFinding, SmellKind};
use crate::spine::ir::{EchoAssertion, EchoShape, FunctionUnit};

pub(super) fn detect(
    ctx: &SmellContext<'_>,
    metrics: &[FunctionUnit],
    config: &Smells,
    out: &mut Vec<SmellFinding>,
) {
    if !ctx.test_source {
        return;
    }
    for metric in metrics {
        for assertion in &metric.review_risks.echo_assertions {
            out.push(echo_finding(ctx, metric, assertion));
        }
    }
    if config.fixture_echo_include_mock_only {
        mock_only_tests(ctx, metrics, config, out);
    }
}

fn echo_finding(
    ctx: &SmellContext<'_>,
    metric: &FunctionUnit,
    assertion: &EchoAssertion,
) -> SmellFinding {
    let sides = match assertion.shape {
        EchoShape::SameExpression => "both sides are the same expression",
        EchoShape::Literals => "both sides are the same literal",
        EchoShape::PassThrough => {
            "both sides read the same field from one fixture through an unchanged alias"
        }
    };
    make(
        SmellKind::FixtureEchoAssertion,
        format!("assertion cannot fail: {sides}; compare the result under test with an independent expected value"),
        ctx.path,
        assertion.line,
        &metric.name,
        Severity::Warning,
        1,
        0,
    )
}

fn mock_only_tests(
    ctx: &SmellContext<'_>,
    metrics: &[FunctionUnit],
    config: &Smells,
    out: &mut Vec<SmellFinding>,
) {
    let assertion_functions = config.assertion_globs();
    let checked = checked_helpers(metrics, assertion_functions);
    for metric in metrics.iter().filter(|metric| {
        checks_only_own_mocks(metric)
            && !calls_check(metric, &checked, assertion_functions)
            && !has_checking_callback(metric, metrics)
    }) {
        out.push(make(
            SmellKind::FixtureEchoAssertion,
            "test only checks that mocks it programmed were called; assert on the value or state the subject produced from them".to_string(),
            ctx.path,
            metric.start_line,
            &metric.name,
            Severity::Info,
            metric.review_risks.test_checks as u32,
            0,
        ));
    }
}

/// Every direct check is an argument-free was-called check on a mock this test programmed.
fn checks_only_own_mocks(metric: &FunctionUnit) -> bool {
    let facts = &metric.review_risks;
    facts.is_test_case
        && facts.test_checks > 0
        && facts.mock_call_checks.len() == facts.test_checks
        && facts
            .mock_call_checks
            .iter()
            .all(|receiver| facts.programmed_mocks.contains(receiver))
}

/// A callback inside the test (`waitFor(() => expect(...))`) holds its own checks.
fn has_checking_callback(test: &FunctionUnit, metrics: &[FunctionUnit]) -> bool {
    metrics.iter().any(|unit| {
        !std::ptr::eq(unit, test)
            && unit.start_line >= test.start_line
            && unit.end_line <= test.end_line
            && unit.review_risks.test_checks > 0
    })
}
