use super::Unit;
use crate::spine::parser::tokens::StructuralToken;
use crate::spine::parser::FunctionUnit;
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

#[derive(PartialEq, Eq, Hash)]
pub(super) struct PairKey {
    left_file: PathBuf,
    left_row: usize,
    right_file: PathBuf,
    right_row: usize,
}

pub(super) fn pair_key(left: &Unit, right: &Unit) -> PairKey {
    let a = (left.file.clone(), left.start);
    let b = (right.file.clone(), right.start);
    if a <= b {
        PairKey {
            left_file: a.0,
            left_row: a.1,
            right_file: b.0,
            right_row: b.1,
        }
    } else {
        PairKey {
            left_file: b.0,
            left_row: b.1,
            right_file: a.0,
            right_row: a.1,
        }
    }
}

pub(super) fn file_hash(path: &Path) -> u64 {
    let mut h = rustc_hash::FxHasher::default();
    path.hash(&mut h);
    if let Ok(bytes) = std::fs::read(path) {
        bytes.hash(&mut h);
    }
    h.finish()
}

pub(super) fn bundle_key(
    file_hash: u64,
    file: &Path,
    symbol_path: &str,
    func: &FunctionUnit,
    tokens: usize,
    shape: &BTreeMap<StructuralToken, usize>,
    comment: &str,
) -> u64 {
    let mut h = rustc_hash::FxHasher::default();
    file_hash.hash(&mut h);
    file.hash(&mut h);
    symbol_path.hash(&mut h);
    func.start_line.hash(&mut h);
    func.end_line.hash(&mut h);
    tokens.hash(&mut h);
    for (token, count) in shape {
        token.hash(&mut h);
        count.hash(&mut h);
    }
    comment.hash(&mut h);
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn func(start: usize, end: usize) -> FunctionUnit {
        FunctionUnit {
            name: "build_payload".to_string(),
            start_line: start,
            end_line: end,
            ..Default::default()
        }
    }

    fn shape(assigns: usize) -> BTreeMap<StructuralToken, usize> {
        let mut shape = BTreeMap::new();
        shape.insert(StructuralToken::FunctionDef, 1);
        shape.insert(StructuralToken::Assign, assigns);
        shape
    }

    #[test]
    fn bundle_key_tracks_semantic_inputs() {
        let base = bundle_key(
            1,
            Path::new("a.py"),
            "a::build_payload",
            &func(1, 8),
            20,
            &shape(3),
            "Build a normalized API payload.",
        );

        assert_ne!(
            base,
            bundle_key(
                2,
                Path::new("a.py"),
                "a::build_payload",
                &func(1, 8),
                20,
                &shape(3),
                "Build a normalized API payload.",
            ),
            "file content changes must invalidate the bundle"
        );
        assert_ne!(
            base,
            bundle_key(
                1,
                Path::new("a.py"),
                "a::build_payload",
                &func(1, 9),
                20,
                &shape(3),
                "Build a normalized API payload.",
            ),
            "symbol range changes must invalidate the bundle"
        );
        assert_ne!(
            base,
            bundle_key(
                1,
                Path::new("a.py"),
                "a::build_payload",
                &func(1, 8),
                21,
                &shape(4),
                "Build a normalized API payload.",
            ),
            "shape changes must invalidate the bundle"
        );
        assert_ne!(
            base,
            bundle_key(
                1,
                Path::new("a.py"),
                "a::build_payload",
                &func(1, 8),
                20,
                &shape(3),
                "Build a normalized admin payload.",
            ),
            "comment changes must invalidate the bundle"
        );
    }
}
