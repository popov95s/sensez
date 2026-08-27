pub(crate) mod javascript;
pub(crate) mod python;
pub(crate) mod rust;

pub(crate) fn lossy_text<'a>(node: tree_sitter::Node<'_>, src: &'a [u8]) -> std::borrow::Cow<'a, str> {
    String::from_utf8_lossy(&src[node.byte_range()])
}
