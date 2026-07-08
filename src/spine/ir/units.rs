//! Per-function / per-class structural summaries consumed by the design-smell
//! pillar. Populated during the language walk (the only place AST nesting and
//! scope are known); languages that don't populate a field leave its default.

use super::PerformanceFacts;
use std::collections::{HashMap, HashSet};

/// Per-function structural summary used by the design-smell pillar.
#[derive(Debug, Clone, Default)]
pub struct FunctionUnit {
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
    /// Parameter names in order (`len()` is the parameter count).
    pub param_names: Vec<String>,
    /// Deepest block nesting within the body (if/for/while/try/with).
    pub max_nesting: usize,
    pub return_count: usize,
    /// Decision points (cyclomatic basis): cyclomatic complexity = 1 + this.
    pub branch_count: usize,
    /// `if x { if y { ... } }` shapes with no else/default path.
    pub collapsible_nested_ifs: usize,
    /// Nesting-weighted cognitive-complexity accumulator (Sonar-style).
    pub cognitive: usize,
    /// Count of "magic" numeric literals (not 0/1/2).
    pub magic_numbers: usize,
    /// Number of source lines occupied by comments inside this function body.
    pub comment_lines: usize,
    /// Longest `a.b.c.d` attribute chain (Law of Demeter / message chains).
    pub max_chain_depth: usize,
    pub is_method: bool,
    /// Local name → number of plain (non-augmented) assignments to it.
    pub local_reassigns: HashMap<String, usize>,
    /// Receiver identifier → number of member accesses on it (feature envy).
    /// Includes `self` so callers can compare external vs. own access.
    pub receiver_access: HashMap<String, usize>,
    /// Set of `self.<attr>` names this function's own body reads/writes (nested
    /// functions excluded). Collected here during the single body walk so the
    /// owning class can assemble its LCOM map without re-walking each method.
    pub self_attrs: HashSet<String>,
    /// Receiver identifier → distinct string-literal subscript keys
    /// (`cfg["host"]`, reads and writes alike) — implicit-schema detection.
    pub str_keys: HashMap<String, HashSet<String>>,
    /// Identifiers whose *object* is mutated in this body: subscript-assign,
    /// `del x[k]`, or a known mutating method call (`x.append(...)`).
    pub mutated_names: HashSet<String>,
    /// Root identifiers mutated *through an attribute* — `m.kwargs[k]=v`,
    /// `m.items.append(...)` record `m`. Kept separate from direct mutation so
    /// the stricter opt-in `param_attr_mutation` smell can use it without
    /// disturbing the default (`self.cache.update()` lands here as `self`,
    /// then is filtered out by the self/cls guard).
    pub attr_mutated_names: HashSet<String>,
    /// Largest arity among bare tuple returns (`return a, b, c`).
    pub max_tuple_return: usize,
    /// Count of `x in ["a", "b", ...]` membership tests against a literal
    /// collection of strings (stringly-typed category logic).
    pub literal_membership_tests: usize,
    /// Source rows for empty/1-char string fallback literals (`or ""`,
    /// `|| "?"`, `cond ? value : "?"`, etc.).
    pub short_string_fallback_lines: Vec<usize>,
    /// Compact call/loop facts consumed by performance smell detectors.
    pub performance: PerformanceFacts,
    /// True if this function is defined inside another function's body
    /// (methods directly on a class are NOT nested).
    pub is_nested: bool,
    /// Name of the enclosing function when `is_nested` (empty otherwise).
    pub parent: String,
}

/// Source span for a comment node. Language profiles record these during the
/// normal syntax walk; the shared walk post-pass attaches them to functions.
#[derive(Debug, Clone, Default)]
pub struct CommentSpan {
    pub start_line: usize,
    pub end_line: usize,
}

/// A smell-detector view of one function: the aggregated metrics + the anchor
/// data (name, line range) needed to build a [`SmellFinding`]. Built from a
/// [`FunctionUnit`] via [`From`], so detectors work against a fixed type
/// instead of reaching into the language-neutral IR's full field set.
#[derive(Debug, Clone, Default)]
pub struct FunctionMetrics {
    // Anchor info — needed to make a `SmellFinding`.
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
    pub param_names: Vec<String>,

    // Categorization — the smell layer routes "is this a method?" and "is
    // this nested?" off the same flags the walker fills.
    pub is_method: bool,
    pub is_nested: bool,
    pub parent: String,

    // Complexity
    pub max_nesting: usize,
    pub return_count: usize,
    pub branch_count: usize,
    pub collapsible_nested_ifs: usize,
    pub cognitive: usize,

    // Size / shape
    pub max_tuple_return: usize,
    pub magic_numbers: usize,
    pub comment_lines: usize,

    // Coupling
    pub max_chain_depth: usize,
    pub receiver_access: HashMap<String, usize>,
    pub str_keys: HashMap<String, HashSet<String>>,

    // Mutation
    pub mutated_names: HashSet<String>,
    pub attr_mutated_names: HashSet<String>,
    pub literal_membership_tests: usize,
    pub local_reassigns: HashMap<String, usize>,
    pub short_string_fallback_lines: Vec<usize>,

    // Performance
    pub performance: PerformanceFacts,
}

impl From<&FunctionUnit> for FunctionMetrics {
    fn from(f: &FunctionUnit) -> Self {
        // Cloning the maps/sets is a small, fixed per-function cost (one
        // smell pass per file, the smell layer already owns the rest of the
        // file's work). We pay it once per function to give the detectors
        // an immutable, owned view that doesn't borrow from the parser IR.
        Self {
            name: f.name.clone(),
            start_line: f.start_line,
            end_line: f.end_line,
            param_names: f.param_names.clone(),
            is_method: f.is_method,
            is_nested: f.is_nested,
            parent: f.parent.clone(),
            max_nesting: f.max_nesting,
            return_count: f.return_count,
            branch_count: f.branch_count,
            collapsible_nested_ifs: f.collapsible_nested_ifs,
            cognitive: f.cognitive,
            max_tuple_return: f.max_tuple_return,
            magic_numbers: f.magic_numbers,
            comment_lines: f.comment_lines,
            max_chain_depth: f.max_chain_depth,
            receiver_access: f.receiver_access.clone(),
            str_keys: f.str_keys.clone(),
            mutated_names: f.mutated_names.clone(),
            attr_mutated_names: f.attr_mutated_names.clone(),
            literal_membership_tests: f.literal_membership_tests,
            local_reassigns: f.local_reassigns.clone(),
            short_string_fallback_lines: f.short_string_fallback_lines.clone(),
            performance: f.performance.clone(),
        }
    }
}

/// Per-class structural summary used by the design-smell pillar.
#[derive(Debug, Clone, Default)]
pub struct ClassUnit {
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
    /// Base-class names (for refused-bequest analysis).
    pub bases: Vec<String>,
    /// True if the class is itself abstract (inherits `ABC`/`Protocol`/`ABCMeta`
    /// or declares any `@abstractmethod`) — abstract stubs are not a smell.
    pub is_abstract: bool,
    pub methods: Vec<String>,
    /// Method name → set of `self.<attr>` it reads or writes (LCOM, temp field).
    pub method_attr_use: HashMap<String, HashSet<String>>,
    /// Concrete methods whose body is only `pass` / `raise NotImplementedError`
    /// (excludes `@abstractmethod`-decorated declarations).
    pub overrides_to_stub: Vec<String>,
    /// Class-level declared properties with known types.
    pub properties: Vec<ClassProperty>,
}

#[derive(Debug, Clone, Default)]
pub struct ClassProperty {
    pub name: String,
    pub type_name: String,
    pub initializer_type: Option<String>,
    pub line: usize,
}

/// Best-effort, annotation-driven type information (no full inference). Absent
/// entries mean "unknown" — type-assisted smells skip rather than guess.
#[derive(Debug, Clone, Default)]
pub struct TypeHints {
    /// (function name, param name) → annotated type text.
    pub param_types: HashMap<(String, String), String>,
    /// local/global variable name → type (annotation or `x = T(...)`).
    pub var_types: HashMap<String, String>,
    /// attribute path → type (for example, `self.model: ApiModel`).
    pub attr_types: HashMap<String, String>,
    /// function name → annotated return type.
    pub return_types: HashMap<String, String>,
}

/// Increment `map[key]`, allocating the key only when absent. On the hot
/// identifier path most occurrences hit an existing entry, so this skips the
/// per-token `String` allocation the `entry(key.to_string())` idiom forces.
pub(crate) fn bump(map: &mut HashMap<String, usize>, key: &str) {
    if let Some(count) = map.get_mut(key) {
        *count += 1;
    } else {
        map.insert(key.to_string(), 1);
    }
}

/// Record that `attr` was accessed on `base`, allocating either string only on
/// first sight (repeat `obj.attr` accesses neither allocate nor re-insert).
pub(crate) fn record_attr(map: &mut HashMap<String, HashSet<String>>, base: &str, attr: &str) {
    match map.get_mut(base) {
        Some(attrs) => {
            if !attrs.contains(attr) {
                attrs.insert(attr.to_string());
            }
        }
        None => {
            let mut attrs = HashSet::new();
            attrs.insert(attr.to_string());
            map.insert(base.to_string(), attrs);
        }
    }
}
