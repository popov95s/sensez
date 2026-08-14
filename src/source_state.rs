//! Shared, compact source identity snapshots.
//!
//! This deliberately contains no parsed syntax, findings, or source bytes.
//! Spine uses the manifest to classify parser-cache reuse; Brainz can retain a
//! complete scan baseline without sharing either subsystem's policy.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SourceManifest {
    files: BTreeMap<PathBuf, u64>,
}

impl SourceManifest {
    pub fn from_hashes(entries: impl IntoIterator<Item = (PathBuf, u64)>) -> Self {
        Self {
            files: entries.into_iter().collect(),
        }
    }

    pub fn from_root_hashes(
        root: &Path,
        entries: impl IntoIterator<Item = (PathBuf, u64)>,
    ) -> Self {
        Self::from_hashes(
            entries
                .into_iter()
                .map(|(path, hash)| (path.strip_prefix(root).unwrap_or(&path).to_path_buf(), hash)),
        )
    }

    pub fn changes(&self, current: &Self) -> ChangeSet {
        let mut changes = ChangeSet::default();
        for (path, hash) in &current.files {
            match self.files.get(path) {
                None => changes.added.push(path.clone()),
                Some(previous) if previous != hash => changes.modified.push(path.clone()),
                Some(_) => changes.unchanged.push(path.clone()),
            }
        }
        changes.deleted = self
            .files
            .keys()
            .filter(|path| !current.files.contains_key(*path))
            .cloned()
            .collect();
        changes
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChangeSet {
    pub added: Vec<PathBuf>,
    pub modified: Vec<PathBuf>,
    pub deleted: Vec<PathBuf>,
    pub unchanged: Vec<PathBuf>,
}

impl ChangeSet {
    pub fn has_changes(&self) -> bool {
        !self.added.is_empty() || !self.modified.is_empty() || !self.deleted.is_empty()
    }
}

/// A source manifest together with the inputs that determine cache validity.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SourceState {
    pub revision: u32,
    pub config_hash: u64,
    pub branch: Option<String>,
    pub manifest: SourceManifest,
}

impl SourceState {
    pub fn new(
        revision: u32,
        config_hash: u64,
        branch: Option<String>,
        manifest: SourceManifest,
    ) -> Self {
        Self {
            revision,
            config_hash,
            branch,
            manifest,
        }
    }

    pub fn changes(&self, current: &Self) -> ChangeSet {
        if self.revision != current.revision
            || self.config_hash != current.config_hash
            || self.branch != current.branch
        {
            return ChangeSet {
                added: current
                    .manifest
                    .files
                    .keys()
                    .filter(|path| !self.manifest.files.contains_key(*path))
                    .cloned()
                    .collect(),
                modified: current
                    .manifest
                    .files
                    .keys()
                    .filter(|path| self.manifest.files.contains_key(*path))
                    .cloned()
                    .collect(),
                deleted: self
                    .manifest
                    .files
                    .keys()
                    .filter(|path| !current.manifest.files.contains_key(*path))
                    .cloned()
                    .collect(),
                ..ChangeSet::default()
            };
        }
        self.manifest.changes(&current.manifest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(entries: &[(&str, u64)]) -> SourceManifest {
        SourceManifest::from_hashes(
            entries
                .iter()
                .map(|(path, hash)| (PathBuf::from(path), *hash)),
        )
    }

    #[test]
    fn classifies_file_changes() {
        let previous = manifest(&[("a.py", 1), ("b.py", 2)]);
        let current = manifest(&[("a.py", 1), ("b.py", 3), ("c.py", 4)]);

        assert_eq!(
            previous.changes(&current),
            ChangeSet {
                added: vec![PathBuf::from("c.py")],
                modified: vec![PathBuf::from("b.py")],
                deleted: Vec::new(),
                unchanged: vec![PathBuf::from("a.py")],
            }
        );
    }

    #[test]
    fn stores_paths_relative_to_the_scan_root() {
        let root = Path::new("/repo");
        let manifest = SourceManifest::from_root_hashes(root, [(root.join("src/lib.rs"), 7)]);

        assert_eq!(manifest.files.get(Path::new("src/lib.rs")), Some(&7));
    }

    #[test]
    fn state_input_change_invalidates_every_file() {
        let previous = SourceState::new(1, 10, Some("main".into()), manifest(&[("a.py", 1)]));
        let current = SourceState::new(1, 11, Some("main".into()), manifest(&[("a.py", 1)]));

        let changes = previous.changes(&current);
        assert_eq!(changes.modified, vec![PathBuf::from("a.py")]);
        assert!(changes.has_changes());
    }
}
