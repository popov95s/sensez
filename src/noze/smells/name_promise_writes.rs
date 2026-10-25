//! Writes a function body performs, classed by how far they reach.

use crate::config::smells::Smells;
use crate::profiles::PerformanceProfile;
use crate::spine::ir::{CallFact, FunctionUnit, StateWriteKind};
use std::collections::HashSet;

/// How far a write reaches beyond the call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Reach {
    /// A memo cache or lazily initialised member.
    Cached,
    /// In-memory state the caller can see: parameters, members, module names.
    Shared,
    /// A database, network or scheduler write.
    Persistent,
}

pub(super) struct Write {
    pub line: usize,
    pub target: String,
    pub reach: Reach,
}

const PERSISTENT_METHODS: &[&str] = &[
    "insert", "patch", "replace", "upsert", "save", "commit", "post", "put",
];
const STORE_METHODS: &[&str] = &["delete", "update"];
const SCHEDULE_METHODS: &[&str] = &["enqueue"];
const CACHE_WORDS: &[&str] = &["cache", "memo"];
const LOOKUP_METHODS: &[&str] = &["get", "has"];

/// A function with the callbacks it passes inline and the functions around it.
pub(super) struct Body<'a> {
    pub unit: &'a FunctionUnit,
    pub callbacks: Vec<&'a FunctionUnit>,
    pub enclosing: Vec<&'a FunctionUnit>,
}

/// Every write in the body that reaches past its own locals.
pub(super) fn writes(
    body: &Body<'_>,
    profile: &dyn PerformanceProfile,
    cfg: &Smells,
) -> Vec<Write> {
    let enclosing: HashSet<&str> = body
        .enclosing
        .iter()
        .flat_map(|unit| local_names(unit))
        .collect();
    let around_callbacks: HashSet<&str> = enclosing
        .iter()
        .copied()
        .chain(local_names(body.unit))
        .collect();
    let mut out: Vec<Write> = std::iter::once((body.unit, &enclosing))
        .chain(
            body.callbacks
                .iter()
                .map(|callback| (*callback, &around_callbacks)),
        )
        .flat_map(|(scope, outer)| {
            let mut locals = local_names(scope);
            locals.extend(outer.iter().copied());
            if !std::ptr::eq(scope, body.unit) {
                locals.extend(scope.param_names.iter().map(String::as_str));
            }
            let calls: Vec<Write> = scope
                .performance
                .calls
                .iter()
                .filter_map(|call| call_write(scope, call, &locals, profile, cfg))
                .collect();
            let assignments: Vec<Write> = assignment_writes(scope, &locals, outer).collect();
            calls.into_iter().chain(assignments)
        })
        .collect();
    out.sort_by_key(|write| write.line);
    out
}

/// Names bound inside `unit`: declarations, assigned names and loop variables.
fn local_names(unit: &FunctionUnit) -> HashSet<&str> {
    let loop_bindings = unit
        .performance
        .loop_frames
        .iter()
        .flat_map(|frame| frame.bindings.iter());
    unit.performance
        .bindings
        .iter()
        .map(|(name, _)| name)
        .chain(unit.local_reassigns.keys())
        .chain(loop_bindings)
        .map(String::as_str)
        .collect()
}

fn call_write(
    unit: &FunctionUnit,
    call: &CallFact,
    locals: &HashSet<&str>,
    profile: &dyn PerformanceProfile,
    cfg: &Smells,
) -> Option<Write> {
    let method = normalized(&call.method);
    let schedules = method.starts_with("schedule")
        || SCHEDULE_METHODS.contains(&method.as_str())
        || profile.scheduler_methods().contains(&method.as_str());
    if !call.member {
        return schedules.then(|| write(call, Reach::Persistent));
    }
    if locals.contains(profile.receiver_root(&call.base)) {
        return None;
    }
    let listed = cfg
        .name_promise_write_methods
        .iter()
        .any(|listed| normalized(listed) == method);
    let mutates = listed || profile.is_mutating_call(&call.method);
    let reach = match () {
        _ if schedules => Reach::Persistent,
        _ if !mutates => return None,
        _ if names_cache(&call.base) => Reach::Cached,
        _ if PERSISTENT_METHODS.contains(&method.as_str()) => Reach::Persistent,
        _ if STORE_METHODS.contains(&method.as_str()) && call.base.contains('.') => {
            Reach::Persistent
        }
        _ if looked_up_before(unit, call) => Reach::Cached,
        _ => Reach::Shared,
    };
    Some(write(call, reach))
}

/// Assignments outside `locals`; the profile already dropped plain names `unit` binds, so
/// plain names are checked only against the functions around it (`outer`).
fn assignment_writes<'a>(
    unit: &'a FunctionUnit,
    locals: &'a HashSet<&str>,
    outer: &'a HashSet<&str>,
) -> impl Iterator<Item = Write> + 'a {
    let is_local = move |target: &str| {
        let root = target.split(['.', '[']).next().unwrap_or(target);
        match root == target {
            true => outer.contains(root),
            false => locals.contains(root),
        }
    };
    unit.state_writes
        .iter()
        .filter(move |state| !is_local(&state.target))
        .map(|state| {
            let reach = match state.kind {
                StateWriteKind::Request => Reach::Persistent,
                StateWriteKind::LazyInit => Reach::Cached,
                _ if names_cache(&state.target) => Reach::Cached,
                StateWriteKind::OwnMember | StateWriteKind::OuterName => Reach::Shared,
            };
            Write {
                line: state.line,
                target: state.target.clone(),
                reach,
            }
        })
}

fn write(call: &CallFact, reach: Reach) -> Write {
    Write {
        line: call.line,
        target: call.target.clone(),
        reach,
    }
}

/// The same receiver was read with `get` / `has` earlier in the body.
fn looked_up_before(unit: &FunctionUnit, write: &CallFact) -> bool {
    unit.performance.calls.iter().any(|call| {
        call.offset < write.offset
            && call.base == write.base
            && LOOKUP_METHODS.contains(&call.method.as_str())
    })
}

fn names_cache(target: &str) -> bool {
    let lower = target.to_ascii_lowercase();
    CACHE_WORDS.iter().any(|word| lower.contains(word))
}

/// Lower-case with `_` removed, so `run_after` and `runAfter` compare equal.
pub(super) fn normalized(name: &str) -> String {
    name.chars()
        .filter(|ch| *ch != '_')
        .flat_map(char::to_lowercase)
        .collect()
}
