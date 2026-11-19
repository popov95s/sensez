use super::occurrences::{dedup_key, occurrence, suppress_overlaps};
#[cfg(feature = "eyez")]
use super::semantic;
use super::{advice, class_shapes, clones, consolidate, flatten, gapped, near_miss, open_tail};
use crate::config::model::Duplication;
use crate::noze::source_scope::SourceScope;
use crate::report::{ActionLevel, CloneClass, CloneDetector, CloneOccurrence, ScanIssue};
use crate::spine::parser::ParsedFile;
use rayon::prelude::*;
use rustc_hash::FxHashSet;
use std::collections::BTreeMap;
use std::path::Path;

#[cfg(test)]
pub fn detect(files: &[ParsedFile], config: &Duplication) -> Vec<CloneClass> {
    detect_with_root(files, config, &SourceScope::for_duplication(config), None).0
}

/// Detect duplication per the config (path excludes, threshold, gap stitching).
pub fn detect_with_root(
    files: &[ParsedFile],
    config: &Duplication,
    scope: &SourceScope,
    root: Option<&Path>,
) -> (Vec<CloneClass>, Vec<ScanIssue>) {
    if files.is_empty() || config.threshold == 0 {
        return (Vec::new(), Vec::new());
    }
    let kept: Vec<&ParsedFile> = files.iter().filter(|f| scope.includes(f)).collect();

    let mut by_family: BTreeMap<crate::spine::ir::Language, Vec<&ParsedFile>> = BTreeMap::new();
    for file in &kept {
        by_family
            .entry(crate::profiles::registry::clone_family(file.language))
            .or_default()
            .push(*file);
    }

    let partitions: Vec<_> = by_family.into_values().collect();
    let ((mut out, mut issues), class_shapes) = rayon::join(
        || {
            partitions
                .into_par_iter()
                .map(|partition| detect_partition(&partition, config, root))
                .reduce(
                    || (Vec::new(), Vec::new()),
                    |(mut classes, mut issues), (more_classes, more_issues)| {
                        classes.extend(more_classes);
                        issues.extend(more_issues);
                        (classes, issues)
                    },
                )
        },
        || {
            class_shapes::detect(
                &kept,
                config.class_name_duplicates,
                config.class_property_overlap_min,
            )
        },
    );
    out.extend(class_shapes);
    out.sort_by_key(|c| {
        (
            c.action,
            std::cmp::Reverse(c.token_length * c.occurrences.len()),
        )
    });
    issues.dedup_by(|a, b| a.message == b.message);
    (out, issues)
}

/// Run the suffix-array clone pipeline over one clone-family file cohort.
fn detect_partition(
    kept: &[&ParsedFile],
    config: &Duplication,
    _root: Option<&Path>,
) -> (Vec<CloneClass>, Vec<ScanIssue>) {
    let mut seen: FxHashSet<u64> = FxHashSet::default();
    let mut out: Vec<CloneClass> = Vec::new();
    let mut master = flatten::build(kept);
    let (seed, nesting) = gapped::seed(config.threshold, config.max_gap);
    let groups = clones::extract(&mut master, seed, nesting);
    let clusters = gapped::stitch(groups, &master.spans, config.max_gap);

    for cluster in clusters
        .into_iter()
        .map(|cluster| open_tail::close_tails(cluster, &master.spans))
        .filter(|cluster| cluster.matched >= config.threshold)
    {
        let cluster_occurrences: Vec<CloneOccurrence> = cluster
            .occ
            .iter()
            .filter_map(|&(start, end)| occurrence(kept, &master.spans, start, end))
            .collect();
        let cluster_occurrences = suppress_overlaps(cluster_occurrences);
        if cluster_occurrences.len() < 2 {
            continue;
        }
        if !seen.insert(dedup_key(&cluster_occurrences, cluster.matched)) {
            continue;
        }
        out.push(CloneClass {
            action: ActionLevel::Advisory,
            token_length: cluster.matched,
            occurrences: cluster_occurrences,
            hint: None,
            detector: CloneDetector::Structural,
            absorbed: Vec::new(),
        });
    }

    for class in near_miss::detect(kept, near_miss::min_tokens(config)) {
        let near_miss_occurrences = suppress_overlaps(class.occurrences);
        if near_miss_occurrences.len() < 2 {
            continue;
        }
        if !seen.insert(dedup_key(&near_miss_occurrences, class.token_length)) {
            continue;
        }
        out.push(CloneClass {
            action: ActionLevel::Advisory,
            occurrences: near_miss_occurrences,
            ..class
        });
    }

    #[cfg(feature = "eyez")]
    let issues = semantic_pass(kept, config, _root, &mut seen, &mut out);
    #[cfg(not(feature = "eyez"))]
    let issues = Vec::new();

    let described = consolidate::consolidate(out)
        .into_iter()
        .map(advice::describe)
        .collect();
    (described, issues)
}

/// Opt-in comment-backed semantic clones; an unusable embedder is a scan issue.
#[cfg(feature = "eyez")]
fn semantic_pass(
    kept: &[&ParsedFile],
    config: &Duplication,
    root: Option<&Path>,
    seen: &mut FxHashSet<u64>,
    out: &mut Vec<CloneClass>,
) -> Vec<ScanIssue> {
    let (classes, issues) = match semantic::detect(kept, &config.semantic, config.threshold, root) {
        Ok(found) => found,
        Err(err) => {
            return vec![ScanIssue {
                stage: crate::report::ScanStage::Analyze,
                file: None,
                message: format!(
                    "duplication.semantic skipped: {err:#}; set [duplication.semantic] \
                     enabled = false to run without it"
                ),
            }]
        }
    };
    for class in classes {
        let semantic_occurrences = suppress_overlaps(class.occurrences);
        if semantic_occurrences.len() < 2 {
            continue;
        }
        if !seen.insert(dedup_key(&semantic_occurrences, class.token_length)) {
            continue;
        }
        out.push(CloneClass {
            occurrences: semantic_occurrences,
            ..class
        });
    }
    issues
}
