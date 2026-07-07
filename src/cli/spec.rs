//! Command-line argument specification.

use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "sensez",
    version,
    about = "Sensez — the structural maintainability layer that complements your linter and type-checker",
    long_about = "Sensez — the structural maintainability layer for your codebase.\n\n\
        Finds the cross-file problems linters and type-checkers can't see: duplication, dead code, \
        import cycles, layering/boundary violations, and design smells. Opinionated guardrails that \
        keep a codebase coherent and maintainable as it grows.\n\n\
        Run it alongside your linter and type-checker (e.g. Ruff/ty for Python, ESLint/tsc for JS/TS), not instead of them."
)]
pub struct Cli {
    /// Root directory of the target project when no subcommand is supplied.
    pub path: Option<PathBuf>,
    #[command(flatten)]
    pub options: ScanOptions,
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Code smell and structure checks. Defaults to scanning the given path.
    Noze(NozeArgs),
    /// First-time setup for a repository.
    Init {
        /// Repository root (default: current directory).
        path: Option<PathBuf>,
        /// Coding agent to configure: claude-code | cursor | other.
        #[arg(long)]
        agent: Option<String>,
        /// Install the Claude Code Stop-gate hook.
        #[arg(long)]
        gate: bool,
        /// Disable local-only usage metrics.
        #[arg(long)]
        no_metrics: bool,
        /// Accept defaults for any question not answered by a flag.
        #[arg(short = 'y', long)]
        yes: bool,
    },
    /// MCP server commands.
    #[cfg(feature = "mcp")]
    Mcp(McpArgs),
    /// Local metrics and value reports.
    #[cfg(feature = "mcp")]
    Brainz(BrainzArgs),
    /// Docs/comment search commands.
    #[cfg(feature = "eyez")]
    Eyez(EyezArgs),
    /// Legacy alias for `sensez noze`.
    #[command(hide = true)]
    Scan {
        path: PathBuf,
        #[command(flatten)]
        options: ScanOptions,
    },
    /// Legacy alias for `sensez noze explain`.
    #[command(hide = true)]
    Explain { term: Option<String> },
    /// Legacy alias for `sensez mcp serve`.
    #[cfg(feature = "mcp")]
    #[command(hide = true)]
    Serve,
    /// Legacy alias for `sensez eyez search`.
    #[cfg(feature = "eyez")]
    #[command(hide = true)]
    Search {
        path: PathBuf,
        query: String,
        #[arg(long, default_value_t = 10)]
        top_k: usize,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Args, Debug)]
#[command(
    after_help = "Examples:\n  sensez noze . --diff --fail-on-new\n  sensez noze . --diff --fail-on-new warning\n  sensez noze . --diff --fail-on-new must_fix",
    args_conflicts_with_subcommands = true,
    subcommand_precedence_over_arg = true
)]
pub struct NozeArgs {
    #[command(subcommand)]
    pub action: Option<NozeAction>,
    /// Root directory of the target project (default: current directory).
    pub path: Option<PathBuf>,
    #[command(flatten)]
    pub options: ScanOptions,
}

#[derive(Subcommand, Debug)]
pub enum NozeAction {
    /// Explicit alias for `sensez noze [PATH]`.
    Sniff {
        /// Root directory of the target project (default: current directory).
        path: Option<PathBuf>,
        #[command(flatten)]
        options: ScanOptions,
    },
    /// Explain a finding category in plain English.
    Explain {
        /// Pillar key or smell kind. Omit to list all.
        term: Option<String>,
    },
}

#[cfg(feature = "mcp")]
#[derive(Args, Debug)]
#[command(subcommand_required = true, arg_required_else_help = true)]
pub struct McpArgs {
    #[command(subcommand)]
    pub action: McpAction,
}

#[cfg(feature = "mcp")]
#[derive(Subcommand, Debug)]
pub enum McpAction {
    /// Run the MCP (Model Context Protocol) server over stdio.
    Serve,
}

#[cfg(feature = "mcp")]
#[derive(Args, Debug)]
#[command(subcommand_required = true, arg_required_else_help = true)]
pub struct BrainzArgs {
    #[command(subcommand)]
    pub action: BrainzAction,
}

#[cfg(feature = "mcp")]
#[derive(Subcommand, Debug)]
pub enum BrainzAction {
    /// Display local-only metrics showing what Sensez helped fix.
    Report {
        /// Repository root (default: current directory).
        path: Option<PathBuf>,
        /// Emit the raw machine-readable metrics payload.
        #[arg(long)]
        json: bool,
    },
}

#[cfg(feature = "eyez")]
#[derive(Args, Debug)]
#[command(subcommand_required = true, arg_required_else_help = true)]
pub struct EyezArgs {
    #[command(subcommand)]
    pub action: EyezAction,
}

#[cfg(feature = "eyez")]
#[derive(Subcommand, Debug)]
pub enum EyezAction {
    /// Semantic search over docstrings + comments.
    Search {
        /// Root directory of the target project.
        path: PathBuf,
        /// Natural-language intent query.
        query: String,
        /// Number of results to return.
        #[arg(long, default_value_t = 10)]
        top_k: usize,
        /// Emit machine-readable JSON instead of a human-readable list.
        #[arg(long)]
        json: bool,
    },
    /// Refresh eyez caches for the repository.
    Reindex {
        /// Root directory of the target project.
        path: PathBuf,
        /// Delete existing eyez caches before rebuilding.
        #[arg(long)]
        force: bool,
        /// Also warm semantic-duplication bundles when configured.
        #[arg(long)]
        semantic: bool,
    },
}

/// Flags shared by every scan invocation.
#[derive(Args, Debug)]
pub struct ScanOptions {
    /// Minimum structural-token run length for the duplication detector.
    #[arg(long)]
    pub threshold: Option<usize>,
    /// Emit aggregated per-rule JSON for configuration tuning.
    #[arg(long)]
    pub summary: bool,
    /// Emit machine-readable JSON instead of a human-readable report.
    #[arg(long)]
    pub json: bool,
    /// Cap each pillar to its top-N ranked findings (0 = unlimited).
    #[arg(long)]
    pub max: Option<usize>,
    /// Print every finding instead of the default top offenders.
    #[arg(long)]
    pub all: bool,
    /// Report only duplicate-code findings.
    #[arg(long = "duplicates")]
    pub duplicates: bool,
    /// Report only high-confidence dead-code findings by default.
    #[arg(long = "dead-code")]
    pub dead_code: bool,
    /// Report only circular-import findings.
    #[arg(long)]
    pub cycles: bool,
    /// Report only boundary violations.
    #[arg(long)]
    pub boundaries: bool,
    /// Report only design-smell findings.
    #[arg(long)]
    pub smells: bool,
    /// Keep only output findings whose source file matches GLOB. Repeatable.
    #[arg(long = "output-glob", alias = "filter", value_name = "GLOB")]
    pub output_glob: Vec<String>,
    /// Keep only findings touching the working-tree diff vs HEAD (uses git).
    #[arg(long)]
    pub diff: bool,
    /// Keep only findings touching a unified diff read from FILE ("-" = stdin).
    #[arg(long, value_name = "FILE", conflicts_with = "diff")]
    pub diff_from: Option<String>,
    /// Exit non-zero if diff-scoped findings meet or exceed the given action level.
    /// Defaults to `must_fix` when the flag is present without a value.
    #[arg(long, value_name = "LEVEL", default_missing_value = "must_fix", num_args = 0..=1)]
    pub fail_on_new: Option<FailOnNewLevel>,
    /// Append a plain-English legend defining each finding category shown.
    #[arg(long)]
    pub explain: bool,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailOnNewLevel {
    #[value(name = "must_fix", alias = "must-fix")]
    MustFix,
    Warning,
    Advisory,
    Info,
}
