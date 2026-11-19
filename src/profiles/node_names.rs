//! Node kind and field lookups from per-grammar tables, skipping tree-sitter's per-call string work.

use rustc_hash::FxHashMap;
use std::num::NonZeroU16;
use std::sync::LazyLock;
use tree_sitter::{Language, Node};

pub(crate) trait NodeNames<'tree> {
    /// Same as [`Node::kind`].
    fn kind_name(&self) -> &'static str;
    /// Same as [`Node::child_by_field_name`].
    fn field(&self, name: &str) -> Option<Node<'tree>>;
}

impl<'tree> NodeNames<'tree> for Node<'tree> {
    fn kind_name(&self) -> &'static str {
        names_for(self)
            .and_then(|names| names.kinds.get(usize::from(self.kind_id())).copied())
            .unwrap_or_else(|| self.kind())
    }

    fn field(&self, name: &str) -> Option<Node<'tree>> {
        match names_for(self) {
            Some(names) => self.child_by_field_id(names.fields.get(name)?.get()),
            None => self.child_by_field_name(name),
        }
    }
}

struct Names {
    language: Language,
    kinds: Vec<&'static str>,
    fields: FxHashMap<&'static str, NonZeroU16>,
}

impl Names {
    fn new(language: Language) -> Self {
        let kinds = (0..language.node_kind_count())
            .map(|id| {
                u16::try_from(id)
                    .ok()
                    .and_then(|id| language.node_kind_for_id(id))
                    .unwrap_or_default()
            })
            .collect();
        let fields = (1..=language.field_count())
            .filter_map(|id| {
                let id = NonZeroU16::new(u16::try_from(id).ok()?)?;
                Some((language.field_name_for_id(id.get())?, id))
            })
            .collect();
        Self {
            language,
            kinds,
            fields,
        }
    }
}

static TABLES: LazyLock<Vec<Names>> = LazyLock::new(|| {
    crate::profiles::registry::grammars()
        .map(Names::new)
        .collect()
});

fn names_for(node: &Node) -> Option<&'static Names> {
    let language = node.language();
    TABLES.iter().find(|names| names.language == *language)
}

#[cfg(all(test, feature = "lang-typescript"))]
#[path = "node_names_tests.rs"]
mod tests;
