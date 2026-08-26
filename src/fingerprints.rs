//! Generic fingerprint primitives for scan-time refreshes.
//!
//! Callers provide typed namespaces, labels, and classes. This module owns the
//! shared shape: stable identity hash, optional content hash, grouped sets, and
//! persisted aging records.

use std::collections::BTreeMap;
use std::fmt::Display;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fingerprint<N, L, C = N> {
    pub hash: u64,
    pub content_hash: u64,
    pub namespace: N,
    pub label: L,
    pub class: C,
}

pub struct RenderedFingerprint {
    pub key: String,
    pub label: String,
    pub class: String,
}

impl<N, L, C> Fingerprint<N, L, C> {
    pub fn identity(hash: u64, namespace: N, label: L, class: C) -> Self {
        Self {
            hash,
            content_hash: hash,
            namespace,
            label,
            class,
        }
    }

    pub fn key(&self) -> String {
        hex(self.hash)
    }

    pub fn rendered(&self) -> RenderedFingerprint
    where
        L: Display,
        C: Display,
    {
        RenderedFingerprint {
            key: self.key(),
            label: self.label.to_string(),
            class: self.class.to_string(),
        }
    }
}

pub type Groups<N, L, C = N> = BTreeMap<N, Vec<Fingerprint<N, L, C>>>;

pub fn hash_parts(parts: &[&str]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for part in parts {
        part.hash(&mut hasher);
    }
    hasher.finish()
}

/// Hash raw bytes for content identity (cache keys and change detection).
pub fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    hasher.write(bytes);
    hasher.finish()
}

pub fn hex(hash: u64) -> String {
    format!("{hash:x}")
}

pub fn class_counts<N, L, C>(groups: &Groups<N, L, C>) -> BTreeMap<String, u64>
where
    C: Display,
{
    groups
        .values()
        .flat_map(|prints| prints.iter())
        .fold(BTreeMap::new(), |mut counts, print| {
            *counts.entry(print.class.to_string()).or_default() += 1;
            counts
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
    enum Ns {
        Source,
    }

    #[test]
    fn identity_defaults_content_to_hash() {
        let print = Fingerprint::identity(42, Ns::Source, "src/lib.rs", Ns::Source);
        assert_eq!(print.content_hash, 42);
        assert_eq!(print.key(), "2a");
    }

    #[test]
    fn hash_parts_is_deterministic_and_order_sensitive() {
        let first = hash_parts(&["a", "b"]);
        assert_eq!(first, hash_parts(&["a", "b"]));
        assert_ne!(first, hash_parts(&["b", "a"]));
        assert_ne!(first, hash_parts(&["ab"]));
        assert_ne!(hash_bytes(b"x = 1\n"), hash_bytes(b"x = 2\n"));
    }
}
