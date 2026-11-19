use crate::spine::ir::ImportContext;

pub(in crate::reflexez) fn context(target: String, source: &str, line: usize) -> ImportContext {
    ImportContext {
        source_module: source.to_string(),
        target_module: target,
        imported_symbols: Vec::new(),
        bindings: Vec::new(),
        binding_phases: Vec::new(),
        line,
        column: 1,
        phase: crate::spine::ir::ImportPhase::Runtime,
        is_inline: true,
        is_module_decl: false,
        is_reexport: false,
        enclosing_scope: None,
    }
}
