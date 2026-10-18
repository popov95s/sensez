use super::Unit;
use crate::spine::parser::FunctionUnit;
use rustc_hash::FxHashSet;
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

pub(super) fn bundle_key(
    file_hash: u64,
    file: &Path,
    symbol_path: &str,
    func: &FunctionUnit,
    tokens: usize,
    shingles: &FxHashSet<u64>,
    comment: &str,
) -> u64 {
    let mut h = rustc_hash::FxHasher::default();
    file_hash.hash(&mut h);
    file.hash(&mut h);
    symbol_path.hash(&mut h);
    func.start_line.hash(&mut h);
    func.end_line.hash(&mut h);
    tokens.hash(&mut h);
    shingles.len().hash(&mut h);
    shingles.iter().fold(0u64, |acc, s| acc ^ s).hash(&mut h);
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

    fn shape(first: u64) -> FxHashSet<u64> {
        FxHashSet::from_iter([first, 7, 9])
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
