use super::NodeNames;

#[test]
fn matches_tree_sitter_for_every_node() {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_typescript::LANGUAGE_TSX.into())
        .unwrap();
    let src = "const a = <B x={1} />; function f(p: number) { return p ?? (a as any); } @@";
    let tree = parser.parse(src, None).unwrap();
    let mut stack = vec![tree.root_node()];
    while let Some(node) = stack.pop() {
        assert_eq!(node.kind_name(), node.kind());
        for name in ["name", "body", "left", "function", "nope"] {
            assert_eq!(node.field(name), node.child_by_field_name(name));
        }
        let mut cursor = node.walk();
        stack.extend(node.children(&mut cursor));
    }
}
