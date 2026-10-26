//! Per-function / per-class structural summaries consumed by the design-smell
//! pillar. Populated during the language walk (the only place AST nesting and
//! scope are known); languages that don't populate a field leave its default.

use super::{MagicNumber, PerformanceFacts, SentinelHandler, StringFallback};
use std::collections::{HashMap, HashSet};

/// Syntax facts used by the high-precision generated-code detectors.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ReviewRiskFacts {
    pub broad_handlers: usize,
    pub empty_fallbacks: usize,
    pub repeated_guards: usize,
    /// Name of a direct callee receiving every parameter unchanged.
    pub forwards_to: Option<String>,
    /// Direct test checks found in this function's own body.
    pub test_checks: usize,
    /// Plain calls to helper functions (framework check roots excluded).
    pub helper_calls: Vec<String>,
    pub test_has_nested_function: bool,
    /// Whether the language profile identified this unit as a test case.
    pub is_test_case: bool,
    /// Handlers that swallow the error and return a placeholder or only log.
    pub sentinel_handlers: Vec<SentinelHandler>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SchemaCall {
    pub target: String,
    pub arguments: Vec<String>,
}

/// The outermost member of one `a.b.c.d` attribute chain.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ChainHit {
    pub line: usize,
    pub depth: usize,
    /// Identifier (or own-instance receiver) the chain starts from.
    pub root: Option<String>,
}

/// One membership test against a literal list of strings.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct LiteralMembership {
    pub line: usize,
    /// Tested value when it is an identifier or member path.
    pub operand: Option<String>,
    pub literals: Vec<String>,
}

/// Receiver shape a mutating method call implies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MutatorFamily {
    Array,
    Collection,
}

/// Whether callers outside the module can reach a function.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Visibility {
    #[default]
    Unknown,
    Private,
    Exported,
}

/// Where a function literal sits relative to a call.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CallbackPosition {
    #[default]
    Standalone,
    /// Passed inline as a call argument (`xs.map((x) => ...)`).
    Argument,
}

/// Enclosing `(conditional node id, arm index)` pairs, outermost first.
pub type BranchPath = Vec<(usize, usize)>;

/// Display name of a scope or function that has none in source.
pub const ANONYMOUS: &str = "<anonymous>";

/// Per-function structural summary used by the design-smell pillar.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct FunctionUnit {
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
    /// Parameter names in order (`len()` is the parameter count).
    pub param_names: Vec<String>,
    /// Deepest block nesting within the body (if/for/while/try/with).
    pub max_nesting: usize,
    pub return_count: usize,
    /// Returns that are the sole statement of a top-level `if` with no else.
    pub guard_returns: usize,
    /// Decision points (cyclomatic basis): cyclomatic complexity = 1 + this.
    pub branch_count: usize,
    /// Outer-`if` lines of `if x { if y { ... } }` shapes with no else/default path.
    pub collapsible_nested_if_lines: Vec<usize>,
    /// Conditional expressions nested inside another conditional expression.
    pub nested_ternaries: Vec<NestedTernary>,
    /// Nesting-weighted cognitive-complexity accumulator (Sonar-style).
    pub cognitive: usize,
    /// "Magic" numeric literals (not 0/1/2) outside always-named contexts.
    pub magic_numbers: Vec<MagicNumber>,
    /// Number of source lines occupied by comments inside this function body.
    pub comment_lines: usize,
    /// Comment lines classed as step-by-step narration.
    pub narration_comment_lines: usize,
    /// Comment lines classed as commented-out code.
    pub code_comment_lines: usize,
    /// First line of a narration or commented-out-code comment.
    pub first_noisy_comment_line: Option<usize>,
    /// Outermost `a.b.c.d` attribute chains (Law of Demeter / message chains).
    pub chain_hits: Vec<ChainHit>,
    pub is_method: bool,
    /// Local name → branch path of each plain (non-augmented) assignment to it.
    pub local_reassigns: HashMap<String, Vec<BranchPath>>,
    /// Receiver identifier (including `self`) → number of member accesses on it.
    pub receiver_access: HashMap<String, usize>,
    /// `self.<attr>` names this function's own body reads or writes.
    pub self_attrs: HashSet<String>,
    /// Calls to methods on the language's own-instance receiver.
    pub own_method_calls: HashSet<String>,
    /// Receiver identifier → distinct string-literal subscript keys (`cfg["host"]`).
    pub str_keys: HashMap<String, HashSet<String>>,
    /// Receiver identifier → line of its first string-literal subscript.
    pub str_key_lines: HashMap<String, usize>,
    /// Direct helper calls with identifier arguments.
    pub schema_calls: Vec<SchemaCall>,
    /// Loose inputs passed through a recognized typed/schema decoder.
    pub validated_names: HashSet<String>,
    /// Domain constructors directly returned by this function.
    pub returned_constructors: HashSet<String>,
    /// Identifiers directly returned by this function (`return x`).
    pub returned_names: HashSet<String>,
    /// Identifiers mutated in place: `x[k] = v`, `del x[k]`, `x.append(...)`.
    pub mutated_names: HashSet<String>,
    /// Root identifiers mutated through an attribute (`m.items.append(...)` records `m`).
    pub attr_mutated_names: HashSet<String>,
    /// Identifiers mutated by a known method call, with the mutator family.
    pub method_mutations: HashSet<(String, MutatorFamily)>,
    /// Largest arity among bare tuple returns (`return a, b, c`).
    pub max_tuple_return: usize,
    /// `x in ["a", "b", ...]` tests against a literal collection of strings.
    pub literal_memberships: Vec<LiteralMembership>,
    /// String literals used as fallbacks (`or ""`, `?? "n/a"`, `x ? x : "?"`).
    pub string_fallbacks: Vec<StringFallback>,
    /// Compact call/loop facts consumed by performance smell detectors.
    pub performance: PerformanceFacts,
    /// Conservative evidence for AI-generated-code patterns.
    pub review_risks: ReviewRiskFacts,
    /// Defined inside another function's body (class methods are not nested).
    pub is_nested: bool,
    /// Name of the enclosing function when `is_nested` (empty otherwise).
    pub parent: String,
    pub callback_position: CallbackPosition,
    pub visibility: Visibility,
    /// The enclosing function is a test-framework callback (`describe`, `it`, hooks).
    pub parent_is_test_callback: bool,
    /// A nested function reads a name bound in its enclosing function.
    pub captures_parent: bool,
    /// Constructor of its class.
    pub is_constructor: bool,
    /// Plain assignments to own-instance members (`this.x = …`).
    pub self_writes: usize,
    /// Receivers also subscripted with a computed (non-literal) key.
    pub dynamic_key_receivers: HashSet<String>,
    /// The tuple return annotation repeats one primitive type.
    pub homogeneous_tuple_return: bool,
}

/// Source span and class of one comment node.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct CommentSpan {
    pub start_line: usize,
    pub end_line: usize,
    pub class: CommentClass,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommentClass {
    /// Directive, markup or rationale: never counted as noise.
    Exempt,
    Code,
    #[default]
    Narration,
}

/// A nested conditional expression; `depth` 0 means nested only across an expression boundary.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct NestedTernary {
    pub line: usize,
    pub root_line: usize,
    pub depth: usize,
}

/// Per-class structural summary used by the design-smell pillar.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ClassUnit {
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
    /// Base-class names (for refused-bequest analysis).
    pub bases: Vec<String>,
    /// Interfaces the class declares it implements.
    pub implements: Vec<String>,
    /// Inherits `ABC`/`Protocol`/`ABCMeta` or declares an `@abstractmethod`.
    pub is_abstract: bool,
    pub methods: Vec<String>,
    /// Method name → set of `self.<attr>` it reads or writes (LCOM, temp field).
    pub method_attr_use: HashMap<String, HashSet<String>>,
    /// Method name → sibling methods invoked through `self` / `this`.
    pub method_calls: HashMap<String, HashSet<String>>,
    /// Non-abstract methods whose body is only `pass` / `raise NotImplementedError`.
    pub overrides_to_stub: Vec<String>,
    /// Class-level declared properties with known types.
    pub properties: Vec<ClassProperty>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ClassProperty {
    pub name: String,
    pub type_name: String,
    pub initializer_type: Option<String>,
    pub line: usize,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct TypeAlias {
    pub name: String,
    pub target: String,
    pub line: usize,
}

/// A function by name and 1-indexed start line (names alone repeat across classes).
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct FunctionKey {
    pub name: String,
    pub start_line: usize,
}

impl FunctionKey {
    pub fn of(func: &FunctionUnit) -> Self {
        FunctionKey {
            name: func.name.clone(),
            start_line: func.start_line,
        }
    }
}

/// Annotation-driven type information; an absent entry means unknown.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct TypeHints {
    /// Function → param name → annotated type text.
    pub param_types: HashMap<FunctionKey, HashMap<String, String>>,
    /// local/global variable name → type (annotation or `x = T(...)`).
    pub var_types: HashMap<String, String>,
    /// attribute path → type (for example, `self.model: ApiModel`).
    pub attr_types: HashMap<String, String>,
    /// Function → annotated return type.
    pub return_types: HashMap<FunctionKey, String>,
    /// Top-level type aliases (`Payload = dict[...]`, `type Payload = ...`).
    pub type_aliases: Vec<TypeAlias>,
}

impl TypeHints {
    pub fn record_param(&mut self, func: &FunctionKey, param: &str, ty: String) {
        self.param_types
            .entry(func.clone())
            .or_default()
            .insert(param.to_string(), ty);
    }

    pub fn record_return(&mut self, func: FunctionKey, ty: String) {
        self.return_types.insert(func, ty);
    }

    pub fn param_type(&self, func: &FunctionUnit, param: &str) -> Option<&String> {
        self.param_types.get(&FunctionKey::of(func))?.get(param)
    }

    pub fn return_type(&self, func: &FunctionUnit) -> Option<&String> {
        self.return_types.get(&FunctionKey::of(func))
    }
}

/// Increment `map[key]`, allocating the key only when absent.
pub(crate) fn bump(map: &mut HashMap<String, usize>, key: &str) {
    if let Some(count) = map.get_mut(key) {
        *count += 1;
    } else {
        map.insert(key.to_string(), 1);
    }
}

/// Record that `attr` was accessed on `base`, allocating only on first sight.
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
