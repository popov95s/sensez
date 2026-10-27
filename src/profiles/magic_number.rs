//! Shared context rules deciding whether a numeric literal needs a name.

use crate::profiles::lineage::Lineage;
use crate::profiles::node_names::NodeNames;
use crate::spine::ir::{MagicContext, MagicNumber};
use tree_sitter::Node;

const ALLOWED: [&str; 5] = ["0", "1", "2", "0.0", "1.0"];
const TIME_FACTORS: [&str; 6] = ["7", "24", "30", "60", "365", "1000"];

/// Per-language literal-context shapes.
pub(crate) struct MagicSyntax {
    pub(crate) number_kinds: &'static [&'static str],
    pub(crate) product_kind: &'static str,
    pub(crate) arguments_kind: &'static str,
    pub(crate) call_kind: &'static str,
    /// Callees whose numeric arguments are delays or formatting widths.
    pub(crate) exempt_callees: &'static [&'static str],
    pub(crate) subscript_kind: &'static str,
    pub(crate) index_field: &'static str,
    /// Names whose comparison, assignment, key or call takes an HTTP status.
    pub(crate) status_names: &'static [&'static str],
    /// Parent kinds pairing a status name with its literal (comparison, assignment, key).
    pub(crate) status_parents: &'static [&'static str],
    /// The literal is the whole initializer of an already-named constant.
    pub(crate) is_named_constant: fn(Lineage, &[u8]) -> bool,
    /// Default value node of one parameter.
    pub(crate) default_value: fn(Node) -> Option<Node>,
}

/// A magic literal at `node`, or `None` when it is allowed or always named by context.
pub(crate) fn magic_number(
    lineage: Lineage,
    src: &[u8],
    syntax: &MagicSyntax,
) -> Option<MagicNumber> {
    let context = context(lineage, src, syntax)?;
    site(lineage.node(), src, context)
}

/// Magic literals in parameter default values (`(retries = 3) => …`).
pub(crate) fn default_values(func: Node, src: &[u8], syntax: &MagicSyntax) -> Vec<MagicNumber> {
    let Some(params) = func.field("parameters") else {
        return Vec::new();
    };
    let mut cursor = params.walk();
    let mut stack: Vec<Node> = params
        .named_children(&mut cursor)
        .filter_map(syntax.default_value)
        .collect();
    let mut out = Vec::new();
    while let Some(node) = stack.pop() {
        if syntax.number_kinds.contains(&node.kind_name()) {
            out.extend(site(node, src, MagicContext::DefaultValue));
        }
        let mut cursor = node.walk();
        stack.extend(node.named_children(&mut cursor));
    }
    out
}

fn site(node: Node, src: &[u8], context: MagicContext) -> Option<MagicNumber> {
    let text = node.utf8_text(src).ok()?;
    (!ALLOWED.contains(&text)).then(|| MagicNumber {
        line: node.start_position().row + 1,
        text: text.to_string(),
        context,
    })
}

fn context(lineage: Lineage, src: &[u8], syntax: &MagicSyntax) -> Option<MagicContext> {
    let node = lineage.node();
    let parent = lineage.parent(node)?;
    if (syntax.is_named_constant)(lineage, src)
        || is_time_product(lineage, src, syntax)
        || callee_matches(lineage, parent, src, syntax, syntax.exempt_callees)
    {
        return None;
    }
    if parent.kind_name() == syntax.subscript_kind
        && parent
            .field(syntax.index_field)
            .is_some_and(|index| index.id() == node.id())
    {
        return Some(MagicContext::ArrayIndex);
    }
    if is_http_status(lineage, parent, src, syntax) {
        return Some(MagicContext::HttpStatus);
    }
    Some(MagicContext::Plain)
}

/// `24 * 60 * 60 * 1000`: every literal factor of the product is a time unit.
fn is_time_product(lineage: Lineage, src: &[u8], syntax: &MagicSyntax) -> bool {
    let node = lineage.node();
    let mut top = node;
    while let Some(parent) = lineage
        .parent(top)
        .filter(|p| p.kind_name() == "parenthesized_expression" || is_product(*p, src, syntax))
    {
        top = parent;
    }
    top.id() != node.id()
        && literal_factors(top, src, syntax)
            .iter()
            .all(|factor| TIME_FACTORS.contains(factor) || ALLOWED.contains(factor))
}

fn literal_factors<'a>(node: Node<'_>, src: &'a [u8], syntax: &MagicSyntax) -> Vec<&'a str> {
    let mut out = Vec::new();
    let mut stack = vec![node];
    while let Some(current) = stack.pop() {
        if syntax.number_kinds.contains(&current.kind_name()) {
            out.push(current.utf8_text(src).unwrap_or_default());
        } else if current.kind_name() == "parenthesized_expression"
            || is_product(current, src, syntax)
        {
            let mut cursor = current.walk();
            stack.extend(current.named_children(&mut cursor));
        }
    }
    out
}

fn is_product(node: Node, src: &[u8], syntax: &MagicSyntax) -> bool {
    node.kind_name() == syntax.product_kind
        && node
            .field("operator")
            .is_some_and(|op| op.utf8_text(src) == Ok("*"))
}

fn is_http_status(lineage: Lineage, parent: Node, src: &[u8], syntax: &MagicSyntax) -> bool {
    let node = lineage.node();
    let in_range = node
        .utf8_text(src)
        .ok()
        .and_then(|text| text.parse::<u16>().ok())
        .is_some_and(|code| (100..=599).contains(&code));
    if !in_range {
        return false;
    }
    if callee_matches(lineage, parent, src, syntax, syntax.status_names) {
        return true;
    }
    if !syntax.status_parents.contains(&parent.kind_name()) {
        return false;
    }
    let mut cursor = parent.walk();
    let mut siblings = parent.named_children(&mut cursor);
    siblings.any(|sibling| {
        sibling.id() != node.id() && ends_with_name(sibling, src, syntax.status_names)
    })
}

fn callee_matches(
    lineage: Lineage,
    arguments: Node,
    src: &[u8],
    syntax: &MagicSyntax,
    names: &[&str],
) -> bool {
    arguments.kind_name() == syntax.arguments_kind
        && lineage
            .parent(arguments)
            .filter(|call| call.kind_name() == syntax.call_kind)
            .and_then(|call| call.field("function"))
            .is_some_and(|callee| ends_with_name(callee, src, names))
}

fn ends_with_name(node: Node, src: &[u8], names: &[&str]) -> bool {
    node.utf8_text(src)
        .ok()
        .is_some_and(|text| names.contains(&super::node_query::last_segment(text).trim()))
}
