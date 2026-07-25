//! Language-neutral module identities used while building the import graph.

use crate::profiles::registry;
use crate::spine::ir::Language;
use crate::spine::parser::ParsedFile;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub(super) struct ModuleIdentity {
    pub(super) root: PathBuf,
    pub(super) logical_name: String,
    pub(super) name: String,
}

pub(super) fn for_files(files: &[ParsedFile], configured: &[PathBuf]) -> Vec<ModuleIdentity> {
    let roots = roots_for(files, configured);
    let logical_names: Vec<_> = files
        .iter()
        .zip(&roots)
        .map(|(file, root)| registry::module_profile(file.language).module_name(&file.path, root))
        .collect();
    let duplicates = duplicate_names(files, &logical_names);
    let workspace = duplicates
        .values()
        .any(|count| *count > 1)
        .then(|| workspace_root(files, configured));

    files
        .iter()
        .zip(roots)
        .zip(logical_names)
        .map(|((file, root), logical_name)| {
            let duplicate = duplicates
                .get(&(file.language, logical_name.clone()))
                .copied()
                .unwrap_or_default()
                > 1;
            let name = workspace
                .as_ref()
                .filter(|_| duplicate)
                .map(|workspace| {
                    registry::module_profile(file.language).disambiguated_module_name(
                        &file.path,
                        workspace,
                        &logical_name,
                    )
                })
                .unwrap_or_else(|| logical_name.clone());
            ModuleIdentity {
                root,
                logical_name,
                name,
            }
        })
        .collect()
}

fn roots_for(files: &[ParsedFile], configured: &[PathBuf]) -> Vec<PathBuf> {
    let mut cache = HashMap::new();
    files
        .iter()
        .map(|file| root_for(file, configured, &mut cache))
        .collect()
}

fn duplicate_names(files: &[ParsedFile], names: &[String]) -> HashMap<(Language, String), usize> {
    let mut counts = HashMap::new();
    for (file, name) in files.iter().zip(names) {
        *counts.entry((file.language, name.clone())).or_insert(0) += 1;
    }
    counts
}

fn workspace_root(files: &[ParsedFile], configured: &[PathBuf]) -> PathBuf {
    if let Some(root) = configured
        .iter()
        .filter(|root| files.iter().all(|file| file.path.starts_with(root)))
        .max_by_key(|root| root.components().count())
    {
        return root.clone();
    }
    let mut directories = files
        .iter()
        .filter_map(|file| file.path.parent().map(Path::to_path_buf));
    let Some(first) = directories.next() else {
        return PathBuf::from(".");
    };
    let common = directories.fold(first, common_ancestor);
    package_root(&common).unwrap_or(common)
}

fn common_ancestor(left: PathBuf, right: PathBuf) -> PathBuf {
    let mut candidate = left;
    while !right.starts_with(&candidate) {
        let Some(parent) = candidate.parent() else {
            break;
        };
        candidate = parent.to_path_buf();
    }
    candidate
}

fn package_root(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .find(|directory| directory.join("package.json").is_file())
        .map(Path::to_path_buf)
}

fn root_for(
    file: &ParsedFile,
    configured: &[PathBuf],
    cache: &mut HashMap<(PathBuf, Language), PathBuf>,
) -> PathBuf {
    if let Some(root) = configured
        .iter()
        .filter(|root| file.path.starts_with(root))
        .max_by_key(|root| root.components().count())
    {
        return root.clone();
    }
    let directory = file.path.parent().unwrap_or(Path::new(".")).to_path_buf();
    cache
        .entry((directory, file.language))
        .or_insert_with(|| registry::module_profile(file.language).root_for(&file.path))
        .clone()
}
