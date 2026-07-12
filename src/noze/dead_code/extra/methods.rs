use super::{usage::ProjectUsage, MemberFiles};
use crate::report::{ActionLevel, Confidence, DeadCodeFinding};
use crate::spine::ir::{ClassUnit, Language};
use crate::spine::parser::{ParsedFile, SymbolKind};
use std::collections::HashSet;
use std::path::PathBuf;

pub(crate) fn unused_methods(
    files: &MemberFiles<'_>,
    modmap: &std::collections::HashMap<PathBuf, String>,
    is_entrypoint_name: impl Fn(Language, &str) -> bool,
) -> Vec<DeadCodeFinding> {
    let usage = ProjectUsage::from_files(files.usage());
    files
        .report()
        .iter()
        .flat_map(|file| unused_methods_in_file(file, modmap, &usage, &is_entrypoint_name))
        .collect()
}

fn unused_methods_in_file(
    file: &ParsedFile,
    modmap: &std::collections::HashMap<PathBuf, String>,
    usage: &ProjectUsage,
    is_entrypoint_name: &impl Fn(Language, &str) -> bool,
) -> Vec<DeadCodeFinding> {
    let module = modmap.get(&file.path).cloned().unwrap_or_default();
    file.walked
        .units
        .classes
        .iter()
        .flat_map(|class| {
            let module = module.clone();
            unique_methods(class).filter_map(move |name| {
                if is_dunder(name) || is_entrypoint_name(file.language, name) {
                    return None;
                }
                let confidence = dead_method_confidence(usage, class, name)?;
                if confidence == Confidence::Low {
                    return None;
                }
                Some(DeadCodeFinding {
                    action: ActionLevel::Advisory,
                    module: module.clone(),
                    symbol: name.to_string(),
                    kind: SymbolKind::Method,
                    confidence,
                    file: file.path.clone(),
                    line: method_line(file, name),
                    reason: String::new(),
                })
            })
        })
        .collect()
}

fn dead_method_confidence(
    usage: &ProjectUsage,
    class: &ClassUnit,
    method: &str,
) -> Option<Confidence> {
    if usage.method_overrides_base(&class.name, method)
        || class
            .method_attr_use
            .values()
            .any(|attrs| attrs.contains(method))
        || usage.typed_member_is_used(&class.name, method)
        || (usage.method_name_is_unique(method) && usage.has_unresolved_member_reference(method))
    {
        return None;
    }
    if usage.has_unresolved_member_reference(method)
        || usage.class_is_exposed_api(&class.name)
        || !usage.class_has_member_evidence(&class.name)
    {
        return Some(Confidence::Low);
    }
    Some(Confidence::High)
}

fn unique_methods(class: &ClassUnit) -> impl Iterator<Item = &str> {
    let mut seen = HashSet::new();
    class
        .methods
        .iter()
        .filter_map(move |name| seen.insert(name.as_str()).then_some(name.as_str()))
}

fn method_line(file: &ParsedFile, method: &str) -> usize {
    file.walked
        .symbols
        .methods
        .iter()
        .find_map(|(name, line)| (name == method).then_some(*line))
        .unwrap_or(0)
}

fn is_dunder(name: &str) -> bool {
    name.starts_with("__") && name.ends_with("__")
}
