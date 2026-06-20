//! MCP tool catalog: name + description + JSON schema for `tools/list`.
//!
//! Sensez records routine metrics automatically and in the background (scans and
//! searches self-record; fixes are recaptured on rescan) — the model is NEVER
//! asked to report, so no tokens are spent on bookkeeping. The only feedback
//! tool is `brainz_triage`, and it is purely user-initiated.

use serde_json::{json, Value};

pub fn tools_list() -> Value {
    #[cfg(feature = "eyez")]
    let tools = {
        let mut tools = vec![
            scan(),
            configuration_summary(),
            gate(),
            explain(),
            triage_finding(),
            usage_report(),
        ];
        tools.insert(1, search_docs());
        tools
    };
    #[cfg(not(feature = "eyez"))]
    let tools = vec![
        scan(),
        configuration_summary(),
        gate(),
        explain(),
        triage_finding(),
        usage_report(),
    ];
    json!({ "tools": tools })
}

fn scan() -> Value {
    json!({
        "name": "noze_sniff",
        "description": "Analyze a codebase for duplication, dead code, import \
                        cycles, boundary violations, and design smells. Run it \
                        after writing or modifying code. ALWAYS pass the \
                        repository root as path — dead-code and cycle analysis \
                        are only correct over the full import graph; scanning a \
                        subdirectory yields false positives. It is fast (<1s on \
                        300k lines), so never scope it down for speed. Fixes \
                        you apply to reported findings are detected \
                        automatically on later rescans. After finishing an \
                        edit session, set diff=true to see only the findings \
                        YOUR uncommitted changes touch — analysis still covers \
                        the whole repo, so diff results are focused but never \
                        wrong.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Absolute path to the REPOSITORY ROOT (not a subdirectory)"},
                "threshold": {"type": "integer", "description": "Duplication token threshold"},
                "limit": {"type": "integer", "description": "Cap each pillar to top-N ranked findings (0 = unlimited)"},
                "diff": {"type": "boolean", "description": "Filter findings to uncommitted working-tree changes vs HEAD (needs git). Full-graph analysis either way."}
            },
            "required": ["path"]
        }
    })
}

fn configuration_summary() -> Value {
    json!({
        "name": "get_configuration_summary",
        "description": "Use this tool when the user explicitly asks to tune, \
                        optimize, or adjust their sensez linting and code-smell \
                        thresholds. It returns a high-level statistical summary \
                        of rule violations across the repository without \
                        flooding the context window.\n\n\
                        Protocol after invoking: analyze which rules generate \
                        the most noise based on counts; present a concise, \
                        conversational summary highlighting the top 2-3 noisy \
                        rules; offer specific adjustments to the .toml \
                        configuration file, such as raising a complexity \
                        threshold or disabling a rule; after user approval, use \
                        normal file-writing capabilities to update the \
                        configuration .toml directly.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Absolute path to the repository root"}
            },
            "required": ["path"]
        }
    })
}

#[cfg(feature = "eyez")]
fn search_docs() -> Value {
    json!({
        "name": "eyez_search_docs",
        "description": "Semantic FIRST-PASS search over a codebase's docstrings \
                        and comments only. A fast way to orient — 'does \
                        something like this already exist?' — before writing new \
                        code. NOT exhaustive and NOT a replacement for a real \
                        search: it ranks by meaning so it can miss relevant code, \
                        and it is blind to undocumented symbols, code bodies, and \
                        anything without a docstring/comment. Treat hits as leads \
                        to confirm, and still use grep/file search when you need \
                        completeness. Usage is recorded automatically; nothing \
                        to report.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Project root to index/search"},
                "query": {"type": "string", "description": "Natural-language intent query"},
                "top_k": {"type": "integer", "description": "Number of results (default 10)"}
            },
            "required": ["path", "query"]
        }
    })
}

/// If `path` sits inside a larger repository, a warning the agent will relay:
/// partial scans produce false dead-code positives and miss cross-module issues.
pub fn scope_warning(path: &std::path::Path) -> Option<String> {
    let path = path.canonicalize().ok()?;
    if path.join(".git").exists() {
        return None; // already the repo root
    }
    let root = path.ancestors().skip(1).find(|a| a.join(".git").exists())?;
    Some(format!(
        "WARNING: '{}' is a subdirectory of the repository at '{}'. Dead-code, \
         cycle, and boundary analysis are only correct over the full import graph \
         — findings below may include false dead-code positives and miss \
         cross-module issues. Rerun with path='{}' unless a partial view is \
         intentional.",
        path.display(),
        root.display(),
        root.display()
    ))
}

fn gate() -> Value {
    json!({
        "name": "noze_gate",
        "description": "End-of-turn quality gate (intended for a Claude Code \
                        Stop hook, not for direct calls): diff-scoped scan of \
                        uncommitted changes; returns hook-decision JSON that \
                        blocks at most once per turn-chain. Findings reported \
                        more than [gate].repeat_limit times on the same source \
                        lines are auto-deferred for the current coding loop; \
                        the first auto-defer expires after three days, while a \
                        second auto-defer for the same lines stays deferred.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Repository root"},
                "stop_hook_active": {"description": "Pass ${stop_hook_active} from the hook input"}
            },
            "required": ["path"]
        }
    })
}

fn explain() -> Value {
    json!({
        "name": "noze_explain",
        "description": "Define a finding category in plain English — a pillar \
                        (duplication, dead_code, cycles, boundaries) or a smell \
                        kind (e.g. inappropriate_intimacy, refused_bequest, \
                        feature_envy). Use it to explain a finding's `kind` to \
                        the user without guessing. Omit `term` to list all. \
                        (Each scan also returns these inline under meta.glossary \
                        for the categories present.)",
        "inputSchema": {
            "type": "object",
            "properties": {
                "term": {"type": "string", "description": "Pillar key or smell kind; omit to list all"}
            }
        }
    })
}

fn triage_finding() -> Value {
    json!({
        "name": "brainz_triage",
        "description": "Record the USER'S verdict on a finding. Call ONLY when \
                        the user explicitly classifies one — never decide on \
                        their behalf, and never call it as routine bookkeeping. \
                        Verdicts (choose by WHY, not by who wrote the code):\n\
                        • 'debt' — the finding is REAL but deliberately not being \
                        fixed now: pre-existing, out of scope, or scheduled \
                        later. \"I didn't write it\" / \"it predates my change\" \
                        is DEBT, not a false positive.\n\
                        • 'false_positive' — Sensez is WRONG: it is not a real \
                        issue (e.g. 'dead' code that is actually invoked \
                        dynamically). Use only when the analysis itself erred.\n\
                        • 'clear' — remove a previous verdict.\n\
                        Stored locally, never exported. Debt-marked findings stop \
                        being surfaced as stale; false positives feed a per-pillar \
                        accuracy signal and are excluded from resolved-value counts.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Project root"},
                "pillar": {
                    "type": "string",
                    "description": "duplication | dead_code | cycles | boundaries | smells"
                },
                "match": {
                    "type": "string",
                    "description": "Case-insensitive substring of the finding label (as shown in scan output / brainz_report.stale_findings)"
                },
                "verdict": {"type": "string", "description": "debt | false_positive | clear"},
                "note": {"type": "string", "description": "Optional one-line user rationale"}
            },
            "required": ["path", "pillar", "match", "verdict"]
        }
    })
}

fn usage_report() -> Value {
    json!({
        "name": "brainz_report",
        "description": "Summarize Sensez' local-only usage metrics for a repo: \
                        this session and all-time (scans run, findings reported \
                        and later resolved, confirmed fixes, searches served, \
                        estimated context saved, stale findings awaiting human \
                        triage). Use when the user asks how \
                        Sensez helped, or before a commit.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Project root to report on"}
            },
            "required": ["path"]
        }
    })
}
