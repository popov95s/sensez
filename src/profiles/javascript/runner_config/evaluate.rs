use super::calls;
use super::defaults;
use super::locate::MODULE_EXTENSIONS;
use super::script::{Import, Script};
use super::value::{Object, Value};
use crate::profiles::node_names::NodeNames;
use std::cell::Cell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tree_sitter::Node;

const MAX_FILE_DEPTH: usize = 8;
const MAX_EVAL_DEPTH: usize = 128;

/// The default export of a config module.
pub fn default_export(path: &Path) -> Value {
    export(path, "default", 0)
}

fn export(path: &Path, name: &str, files: usize) -> Value {
    if files > MAX_FILE_DEPTH {
        return Value::Opaque("config imports nest too deeply".into());
    }
    if path.extension().is_some_and(|ext| ext == "json") {
        return json_export(path);
    }
    let Some(script) = Script::load(path) else {
        return Value::Opaque(format!("`{}` cannot be parsed", path.display()));
    };
    let index = script.index();
    let node = match name {
        "default" => index.default_export,
        _ => index.bindings.get(name).copied(),
    };
    let Some(node) = node else {
        return Value::Opaque(format!("`{}` has no `{name}` export", path.display()));
    };
    let evaluator = Evaluator {
        script: &script,
        imports: &index.imports,
        files,
        depth: Cell::new(0),
    };
    let scope = Scope {
        names: index.bindings.clone(),
        parent: None,
    };
    evaluator.eval(node, &scope)
}

fn json_export(path: &Path) -> Value {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .map_or_else(
            || Value::Opaque(format!("`{}` is not valid JSON", path.display())),
            |json| Value::from_json(&json),
        )
}

/// Names visible at one block level.
pub struct Scope<'p, 't> {
    names: HashMap<String, Node<'t>>,
    parent: Option<&'p Scope<'p, 't>>,
}

impl<'t> Scope<'_, 't> {
    fn lookup(&self, name: &str) -> Option<Node<'t>> {
        self.names
            .get(name)
            .copied()
            .or_else(|| self.parent.and_then(|parent| parent.lookup(name)))
    }
}

pub struct Evaluator<'t> {
    pub script: &'t Script,
    imports: &'t HashMap<String, Import>,
    files: usize,
    depth: Cell<usize>,
}

impl<'t> Evaluator<'t> {
    pub fn eval(&self, node: Node<'t>, scope: &Scope<'_, 't>) -> Value {
        if self.depth.get() > MAX_EVAL_DEPTH {
            return Value::Opaque("config expression nests too deeply".into());
        }
        self.depth.set(self.depth.get() + 1);
        let value = self.eval_node(node, scope);
        self.depth.set(self.depth.get() - 1);
        value
    }

    pub fn directory(&self) -> PathBuf {
        self.script
            .path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default()
    }

    /// Default export of the module a relative specifier names.
    pub fn follow(&self, specifier: &str, name: &str) -> Value {
        match resolve_module(&self.directory(), specifier) {
            Some(path) => export(&path, name, self.files + 1),
            None => Value::Opaque(format!("`{specifier}` does not resolve to a config file")),
        }
    }

    fn eval_node(&self, node: Node<'t>, scope: &Scope<'_, 't>) -> Value {
        match node.kind_name() {
            "string" => self.script.string(node).map_or(Value::Other, Value::Str),
            "template_string" => self.template(node, scope),
            "regex" => node.field("pattern").map_or(Value::Other, |pattern| {
                Value::Regex(self.script.text(pattern))
            }),
            "true" => Value::Bool(true),
            "false" => Value::Bool(false),
            "number" | "null" | "undefined" => Value::Other,
            "array" => self.array(node, scope),
            "object" => self.object(node, scope),
            "identifier" => self.identifier(node, scope),
            "member_expression" => self.member(node, scope),
            "call_expression" => calls::evaluate(self, node, scope),
            "arrow_function" | "function_expression" | "function_declaration" => {
                self.function_result(node, scope)
            }
            "parenthesized_expression"
            | "as_expression"
            | "satisfies_expression"
            | "non_null_expression"
            | "await_expression" => node
                .named_child(0)
                .map_or(Value::Other, |inner| self.eval(inner, scope)),
            "binary_expression" => self.concatenation(node, scope),
            _ => self.unreadable(node),
        }
    }

    pub fn unreadable(&self, node: Node) -> Value {
        let text = self.script.text(node);
        let snippet: String = text.chars().take(40).collect();
        Value::Opaque(format!("`{snippet}` is not statically readable"))
    }

    fn identifier(&self, node: Node<'t>, scope: &Scope<'_, 't>) -> Value {
        let name = self.script.text(node);
        if let Some(bound) = scope.lookup(&name) {
            return self.eval(bound, scope);
        }
        if let Some(import) = self.imports.get(&name) {
            return self.imported(import);
        }
        match name.as_str() {
            "__dirname" => Value::Str(self.directory().to_string_lossy().into_owned()),
            "undefined" => Value::Other,
            _ => Value::Opaque(format!("`{name}` is not a module-level constant")),
        }
    }

    fn imported(&self, import: &Import) -> Value {
        if import.source.starts_with('.') {
            return self.follow(&import.source, &import.name);
        }
        if import.source == "vitest" || import.source.starts_with("vitest/") {
            if let Some(value) = defaults::vitest_export(&import.name) {
                return value;
            }
        }
        Value::Opaque(format!(
            "`{}` is imported from `{}`",
            import.name, import.source
        ))
    }

    fn member(&self, node: Node<'t>, scope: &Scope<'_, 't>) -> Value {
        let text = self.script.text(node);
        if text == "import.meta.dirname" {
            return Value::Str(self.directory().to_string_lossy().into_owned());
        }
        if text.starts_with("process.env") {
            return Value::Opaque(format!("`{text}` depends on the environment"));
        }
        let (Some(object), Some(property)) = (node.field("object"), node.field("property")) else {
            return self.unreadable(node);
        };
        self.eval(object, scope)
            .get(&self.script.text(property))
            .unwrap_or(Value::Other)
    }

    fn array(&self, node: Node<'t>, scope: &Scope<'_, 't>) -> Value {
        let mut items = Vec::new();
        for child in named_children(node) {
            if child.kind_name() != "spread_element" {
                items.push(self.eval(child, scope));
                continue;
            }
            match child.named_child(0).map(|inner| self.eval(inner, scope)) {
                Some(Value::Array(spread)) => items.extend(spread),
                Some(Value::Opaque(reason)) => items.push(Value::Opaque(reason)),
                _ => items.push(self.unreadable(child)),
            }
        }
        Value::Array(items)
    }

    fn object(&self, node: Node<'t>, scope: &Scope<'_, 't>) -> Value {
        let mut object = Object::default();
        for child in named_children(node) {
            match child.kind_name() {
                "pair" => {
                    let key = child.field("key");
                    let value = child.field("value");
                    let name = key.and_then(|key| self.property_name(key));
                    match (name, value) {
                        (Some(name), Some(value)) => {
                            object.entries.push((name, self.eval(value, scope)))
                        }
                        _ => object.unknown = Some("object has a computed key".into()),
                    }
                }
                "shorthand_property_identifier" => object
                    .entries
                    .push((self.script.text(child), self.identifier(child, scope))),
                "spread_element" => {
                    match child.named_child(0).map(|inner| self.eval(inner, scope)) {
                        Some(Value::Object(spread)) => {
                            object.entries.extend(spread.entries);
                            object.unknown = object.unknown.or(spread.unknown);
                        }
                        Some(Value::Opaque(reason)) => object.unknown = Some(reason),
                        _ => {}
                    }
                }
                "method_definition" => {
                    if let Some(name) = child.field("name") {
                        object.entries.push((self.script.text(name), Value::Other));
                    }
                }
                _ => {}
            }
        }
        Value::Object(object)
    }

    fn property_name(&self, key: Node) -> Option<String> {
        match key.kind_name() {
            "property_identifier" | "number" => Some(self.script.text(key)),
            "string" => self.script.string(key),
            _ => None,
        }
    }

    fn function_result(&self, node: Node<'t>, scope: &Scope<'_, 't>) -> Value {
        let Some(body) = node.field("body") else {
            return self.unreadable(node);
        };
        if body.kind_name() != "statement_block" {
            return self.eval(body, scope);
        }
        let statements = named_children(body);
        let local = Scope {
            names: statements
                .iter()
                .flat_map(|statement| self.script.declarations(*statement))
                .collect(),
            parent: Some(scope),
        };
        statements
            .iter()
            .rev()
            .find(|statement| statement.kind_name() == "return_statement")
            .and_then(|statement| statement.named_child(0))
            .map_or_else(
                || Value::Opaque("config function has no top-level return".into()),
                |returned| self.eval(returned, &local),
            )
    }

    fn template(&self, node: Node<'t>, scope: &Scope<'_, 't>) -> Value {
        let mut text = String::new();
        for part in named_children(node) {
            match part.kind_name() {
                "template_substitution" => {
                    match part.named_child(0).map(|inner| self.eval(inner, scope)) {
                        Some(Value::Str(value)) => text.push_str(&value),
                        _ => return self.unreadable(part),
                    }
                }
                _ => text.push_str(&self.script.text(part)),
            }
        }
        Value::Str(text)
    }

    fn concatenation(&self, node: Node<'t>, scope: &Scope<'_, 't>) -> Value {
        let operands = (
            node.field("left"),
            node.field("operator"),
            node.field("right"),
        );
        let (Some(left), Some(operator), Some(right)) = operands else {
            return self.unreadable(node);
        };
        if self.script.text(operator) != "+" {
            return self.unreadable(node);
        }
        match (self.eval(left, scope), self.eval(right, scope)) {
            (Value::Str(left), Value::Str(right)) => Value::Str(left + &right),
            _ => self.unreadable(node),
        }
    }
}

/// Named children without comments.
pub fn named_children(node: Node) -> Vec<Node> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .filter(|child| child.kind_name() != "comment")
        .collect()
}

fn resolve_module(directory: &Path, specifier: &str) -> Option<PathBuf> {
    let base = directory.join(specifier);
    if base.is_file() {
        return Some(base);
    }
    let stem = match base.extension().and_then(|ext| ext.to_str()) {
        Some("js" | "mjs" | "cjs") => base.with_extension(""),
        _ => base.clone(),
    };
    MODULE_EXTENSIONS
        .iter()
        .map(|ext| PathBuf::from(format!("{}.{ext}", stem.display())))
        .chain(
            MODULE_EXTENSIONS
                .iter()
                .map(|ext| base.join(format!("index.{ext}"))),
        )
        .find(|candidate| candidate.is_file())
}
