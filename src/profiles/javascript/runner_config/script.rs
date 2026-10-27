use crate::profiles::node_names::NodeNames;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tree_sitter::{Node, Tree};

/// A parsed config module.
pub struct Script {
    pub path: PathBuf,
    pub source: Vec<u8>,
    tree: Tree,
}

/// A binding imported from another module.
pub struct Import {
    pub source: String,
    pub name: String,
}

/// Top-level names of a config module.
pub struct ModuleIndex<'t> {
    pub bindings: HashMap<String, Node<'t>>,
    pub imports: HashMap<String, Import>,
    pub default_export: Option<Node<'t>>,
}

impl Script {
    pub fn load(path: &Path) -> Option<Self> {
        let source = std::fs::read(path).ok()?;
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&crate::profiles::javascript::grammar::language(path))
            .ok()?;
        let tree = parser.parse(&source, None)?;
        Some(Self {
            path: path.to_path_buf(),
            source,
            tree,
        })
    }

    pub fn index(&self) -> ModuleIndex<'_> {
        let mut index = ModuleIndex {
            bindings: HashMap::new(),
            imports: HashMap::new(),
            default_export: None,
        };
        let root = self.tree.root_node();
        let mut cursor = root.walk();
        for statement in root.named_children(&mut cursor) {
            self.index_statement(statement, &mut index);
        }
        index
    }

    pub fn text(&self, node: Node) -> String {
        node.utf8_text(&self.source).unwrap_or_default().to_string()
    }

    pub fn string(&self, node: Node) -> Option<String> {
        crate::profiles::javascript::const_eval::evaluate(node, &self.source, &HashMap::new())
            .filter(|_| node.kind_name() == "string")
    }

    fn index_statement<'t>(&'t self, statement: Node<'t>, index: &mut ModuleIndex<'t>) {
        match statement.kind_name() {
            "import_statement" => self.index_import(statement, index),
            "export_statement" => {
                if let Some(value) = statement.field("value") {
                    index.default_export = Some(value);
                } else if let Some(declaration) = statement.field("declaration") {
                    self.index_statement(declaration, index);
                }
            }
            "expression_statement" => {
                let Some(assignment) = statement
                    .named_child(0)
                    .filter(|node| node.kind_name() == "assignment_expression")
                else {
                    return;
                };
                let left = assignment.field("left");
                if left.is_some_and(|left| self.text(left) == "module.exports") {
                    index.default_export = assignment.field("right");
                }
            }
            _ => index.bindings.extend(self.declarations(statement)),
        }
    }

    /// Names declared by a `const`/`let`/`var`/`function` statement.
    pub fn declarations<'t>(&'t self, statement: Node<'t>) -> Vec<(String, Node<'t>)> {
        match statement.kind_name() {
            "lexical_declaration" | "variable_declaration" => {
                let mut cursor = statement.walk();
                statement
                    .named_children(&mut cursor)
                    .filter(|child| child.kind_name() == "variable_declarator")
                    .filter_map(|declarator| {
                        let name = declarator.field("name")?;
                        let value = declarator.field("value")?;
                        (name.kind_name() == "identifier").then(|| (self.text(name), value))
                    })
                    .collect()
            }
            "function_declaration" => statement
                .field("name")
                .map(|name| vec![(self.text(name), statement)])
                .unwrap_or_default(),
            _ => Vec::new(),
        }
    }

    fn index_import(&self, statement: Node, index: &mut ModuleIndex) {
        let Some(source) = statement.field("source").and_then(|node| self.string(node)) else {
            return;
        };
        let mut cursor = statement.walk();
        let clauses: Vec<_> = statement
            .named_children(&mut cursor)
            .filter(|child| child.kind_name() == "import_clause")
            .collect();
        for clause in clauses {
            let mut clause_cursor = clause.walk();
            for part in clause.named_children(&mut clause_cursor) {
                match part.kind_name() {
                    "identifier" => {
                        index.imports.insert(
                            self.text(part),
                            Import {
                                source: source.clone(),
                                name: "default".into(),
                            },
                        );
                    }
                    "named_imports" => {
                        let mut named_cursor = part.walk();
                        for specifier in part.named_children(&mut named_cursor) {
                            let Some(name) = specifier.field("name") else {
                                continue;
                            };
                            let local = specifier.field("alias").unwrap_or(name);
                            index.imports.insert(
                                self.text(local),
                                Import {
                                    source: source.clone(),
                                    name: self.text(name),
                                },
                            );
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
