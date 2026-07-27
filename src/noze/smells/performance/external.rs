use super::super::SmellContext;
use crate::profiles::PerformanceProfile;
use crate::spine::ir::{CallFact, FunctionMetrics};
use std::collections::BTreeMap;

pub(super) fn external_calls<'a>(
    ctx: &SmellContext<'_>,
    owner: &FunctionMetrics,
    calls: &'a [CallFact],
    functions: &BTreeMap<&str, &FunctionMetrics>,
    profile: &dyn PerformanceProfile,
) -> BTreeMap<&'a str, &'a CallFact> {
    let mut out = BTreeMap::new();
    for call in calls {
        if functions.contains_key(call.target.as_str()) {
            continue;
        }
        if is_external(ctx, owner, call, profile) {
            out.entry(call.target.as_str()).or_insert(call);
        }
    }
    out
}

fn is_external(
    ctx: &SmellContext<'_>,
    owner: &FunctionMetrics,
    call: &CallFact,
    profile: &dyn PerformanceProfile,
) -> bool {
    if !call.member {
        return false;
    }
    let root = profile.receiver_root(&call.base);
    let receiver_hint = ctx
        .type_hints
        .param_types
        .get(&(owner.name.clone(), root.to_string()))
        .or_else(|| ctx.type_hints.var_types.get(root));
    profile.is_external_loop_call(
        &call.method,
        root,
        receiver_hint.map(String::as_str),
        &owner.performance.loops,
    )
}
