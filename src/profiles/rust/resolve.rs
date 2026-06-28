//! Resolve a Rust `use` path to a module key.
//!
//! `crate::` resolves against the importer's crate source dir (the nearest
//! ancestor holding `lib.rs`/`main.rs`), `self::`/`super::` against the
//! importer's own module path, and the package's own name (from the root
//! `Cargo.toml`, hyphens normalized) like `crate` — so `use sensez::cli` in
//! `main.rs` or an integration test resolves to the in-repo lib module.
//! Everything else (`std::…`, third-party crates) is returned unchanged and
//! becomes an external node.

use super::roots;
use crate::spine::ir::ImportContext;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// The package (directory key) containing `module_name`. An index module
/// (`mod.rs`/`lib.rs`) is its own package; otherwise drop the last segment.
pub fn containing_package(module_name: &str, is_index: bool) -> String {
    if is_index {
        return module_name.to_string();
    }
    match module_name.rsplit_once('/') {
        Some((pkg, _)) => pkg.to_string(),
        None => String::new(),
    }
}

/// Resolve `import.target_module` (raw `::` path) to a module key.
pub fn resolve_target(import: &ImportContext, importer_file: &Path, root: &Path) -> String {
    let target = &import.target_module;
    let mut segments = target.split("::");
    let Some(first) = segments.next() else {
        return target.clone();
    };
    let rest: Vec<&str> = segments.collect();
    match first {
        "crate" => join_key(src_key(importer_file, root), &rest),
        "self" => join_key(roots::module_name(importer_file, root), &rest),
        "super" => resolve_super(importer_file, root, &rest),
        name if Some(name) == package_name(root).as_deref() => {
            join_key(src_key(importer_file, root), &rest)
        }
        name => {
            // Rust 2018 uniform path: a bare first segment may be a sibling
            // submodule in scope (`use reachability::…` next to a
            // `mod reachability;`). If the child module file exists on disk,
            // resolve it like `self::…`; otherwise it's an external crate.
            if child_module_exists(importer_file, name) {
                let module = roots::module_name(importer_file, root);
                return join_key(join_key(module, &[name]), &rest);
            }
            target.clone() // `std::…` / third-party crate → external node
        }
    }
}

fn resolve_super(importer_file: &Path, root: &Path, rest: &[&str]) -> String {
    let mut state = SuperPath::new(roots::module_name(importer_file, root), rest);
    while state.consume_super() {}
    join_key(state.module, state.rest)
}

struct SuperPath<'a> {
    module: String,
    rest: &'a [&'a str],
}

impl<'a> SuperPath<'a> {
    fn new(module: String, rest: &'a [&'a str]) -> Self {
        Self { module, rest }
    }

    fn consume_super(&mut self) -> bool {
        self.module = containing_package(&self.module, false);
        if matches!(self.rest.first(), Some(&"super")) {
            self.rest = &self.rest[1..];
            true
        } else {
            false
        }
    }
}

/// True if `name` is a child module file of the importer: `name.rs` or
/// `name/mod.rs` in the importer's module directory (its own dir for an index
/// file `mod.rs`/`lib.rs`/`main.rs`, else a dir named after its stem).
fn child_module_exists(importer_file: &Path, name: &str) -> bool {
    let Some(parent) = importer_file.parent() else {
        return false;
    };
    let dir = match importer_file.file_stem().and_then(|s| s.to_str()) {
        Some("mod" | "lib" | "main") => parent.to_path_buf(),
        Some(stem) => parent.join(stem),
        None => return false,
    };
    dir.join(format!("{name}.rs")).exists() || dir.join(name).join("mod.rs").exists()
}

fn join_key(base: String, rest: &[&str]) -> String {
    if rest.is_empty() {
        return base;
    }
    if base.is_empty() {
        return rest.join("/");
    }
    format!("{base}/{}", rest.join("/"))
}

/// Key of the importer's crate source dir: the nearest ancestor (up to `root`)
/// containing `lib.rs` or `main.rs`, else `root/src` when that is a crate dir
/// (covers `tests/`/`benches/`, which are sibling crates of `src/`), else the
/// root itself.
fn src_key(importer_file: &Path, root: &Path) -> String {
    let mut dir = importer_file.parent();
    while let Some(d) = dir {
        if !d.starts_with(root) {
            break;
        }
        if d.join("lib.rs").exists() || d.join("main.rs").exists() {
            return rel_key(d, root);
        }
        dir = d.parent();
    }
    let src = root.join("src");
    if src.join("lib.rs").exists() || src.join("main.rs").exists() {
        return rel_key(&src, root);
    }
    String::new()
}

fn rel_key(dir: &Path, root: &Path) -> String {
    dir.strip_prefix(root)
        .unwrap_or(dir)
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// The package name from `root/Cargo.toml` (hyphens → underscores, matching
/// the crate name `use` paths carry), memoized per root — graph building calls
/// this once per import, and the manifest never changes mid-scan.
fn package_name(root: &Path) -> Option<String> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, Option<String>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Mutex::default);
    let mut cache = match cache.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    cache
        .entry(root.to_path_buf())
        .or_insert_with(|| read_package_name(root))
        .clone()
}

fn read_package_name(root: &Path) -> Option<String> {
    let text = std::fs::read_to_string(root.join("Cargo.toml")).ok()?;
    let manifest: toml::Value = text.parse().ok()?;
    let name = manifest.get("package")?.get("name")?.as_str()?;
    Some(name.replace('-', "_"))
}
