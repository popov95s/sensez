use super::super::SmellContext;
use super::loop_context::owned_by_loop;
use crate::config::smells::Smells;
use crate::globs::compile_validated;
use crate::profiles::PerformanceProfile;
use crate::spine::ir::{CallFact, FunctionUnit, LoopKind};
use globset::GlobSet;
use std::collections::BTreeMap;

/// `n_plus_one_call` knobs resolved once per detect.
pub(super) struct LoopCallRules {
    callees: GlobSet,
    await_in_loop: bool,
}

impl LoopCallRules {
    pub(super) fn new(cfg: &Smells) -> Self {
        LoopCallRules {
            callees: compile_validated(&cfg.n_plus_one_callees),
            await_in_loop: cfg.n_plus_one_await_in_loop,
        }
    }
}

pub(super) fn external_calls<'a>(
    ctx: &SmellContext<'_>,
    owner: &FunctionUnit,
    calls: impl IntoIterator<Item = &'a CallFact>,
    functions: &BTreeMap<&str, Vec<&FunctionUnit>>,
    profile: &dyn PerformanceProfile,
    rules: &LoopCallRules,
) -> BTreeMap<&'a str, &'a CallFact> {
    let mut out = BTreeMap::new();
    for call in calls {
        if functions.contains_key(call.target.as_str()) {
            continue;
        }
        if is_external(ctx, owner, call, profile, rules) {
            out.entry(call.target.as_str()).or_insert(call);
        }
    }
    out
}

fn is_external(
    ctx: &SmellContext<'_>,
    owner: &FunctionUnit,
    call: &CallFact,
    profile: &dyn PerformanceProfile,
    rules: &LoopCallRules,
) -> bool {
    if rules.callees.is_match(&call.target) {
        return true;
    }
    if !call.member {
        return false;
    }
    let root = profile.receiver_root(&call.base);
    let receiver_hint = ctx
        .type_hints
        .param_type(owner, root)
        .or_else(|| ctx.type_hints.var_types.get(root));
    profile.is_external_loop_call(
        &call.method,
        root,
        receiver_hint.map(String::as_str),
        &owner.performance.loops,
    ) || (rules.await_in_loop && awaits_outside_item(owner, call, root))
}

/// Awaited per-item call on a receiver that does not come from the item itself.
fn awaits_outside_item(owner: &FunctionUnit, call: &CallFact, root: &str) -> bool {
    let facts = &owner.performance;
    call.awaited
        && facts
            .frames(call.frame)
            .next()
            .is_some_and(|frame| matches!(frame.kind, LoopKind::ForOf | LoopKind::IterCallback))
        && !owned_by_loop(facts, call.frame, root)
}
