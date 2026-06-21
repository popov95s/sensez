//! Data Clumps: groups of parameters that recur together across signatures.
//!
//! Scoped per file (a clump is refactored within its module): mines fixed-size
//! parameter combinations appearing in `min_occurrences`+ signatures of one
//! file, then **merges overlapping combinations** so a recurring bundle of N
//! fields is reported once (as the union) instead of once per size-k subset —
//! a sign the fields want to become one object. Per-file scoping also avoids
//! ubiquitous fields (`id`, `org_id`) chaining unrelated bundles together.

use super::union_find::{find, union};
use super::{make, structure_target};
use crate::config::smells::{SmellConfig, Smells};
use crate::noze::{Severity, SmellFinding, SmellKind};
use crate::spine::parser::ParsedFile;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

/// Cap params considered per function (bounds combination blow-up).
const MAX_PARAMS_CONSIDERED: usize = 10;

struct Seed {
    /// Distinct functions (by index) whose signature contains this combination.
    funcs: BTreeSet<usize>,
    file: PathBuf,
    line: usize,
}

pub fn detect(files: &[&ParsedFile], cfg: &SmellConfig) -> Vec<SmellFinding> {
    files
        .iter()
        .flat_map(|f| detect_in_file(f, cfg.for_language(f.language)))
        .collect()
}

fn detect_in_file(file: &ParsedFile, cfg: &Smells) -> Vec<SmellFinding> {
    if cfg.disabled.contains(&SmellKind::DataClump) {
        return Vec::new();
    }
    let size = cfg.data_clump_min_fields.max(2);
    let mut seeds: BTreeMap<Vec<String>, Seed> = BTreeMap::new();

    for (fn_id, func) in file.walked.units.functions.iter().enumerate() {
        let mut params: Vec<String> = func
            .param_names
            .iter()
            .filter(|p| !matches!(p.as_str(), "self" | "cls"))
            .cloned()
            .collect();
        params.sort();
        params.dedup();
        params.truncate(MAX_PARAMS_CONSIDERED);
        if params.len() < size {
            continue;
        }
        for combo in combinations(&params, size) {
            seeds
                .entry(combo)
                .and_modify(|s| {
                    s.funcs.insert(fn_id);
                })
                .or_insert_with(|| Seed {
                    funcs: BTreeSet::from([fn_id]),
                    file: file.path.clone(),
                    line: func.start_line,
                });
        }
    }

    // Keep only recurring combinations, then merge ones that overlap (a recurring
    // 5-field bundle yields many size-3 seeds sharing >= size-1 fields).
    let qualifying: Vec<(Vec<String>, Seed)> = seeds
        .into_iter()
        .filter(|(_, s)| s.funcs.len() >= cfg.data_clump_min_occurrences)
        .collect();

    merge_overlapping(qualifying, size)
        .into_iter()
        .map(|group| {
            let fields: Vec<String> = group.fields.into_iter().collect();
            make(
                SmellKind::DataClump,
                format!(
                    "fields ({}) recur together across {} signatures — consider {}",
                    fields.join(", "),
                    group.support,
                    structure_target(file.language)
                ),
                &group.file,
                group.line,
                &fields.join("+"),
                Severity::Info,
                group.support as u32,
                cfg.data_clump_min_occurrences as u32,
            )
        })
        .collect()
}

struct Group {
    fields: BTreeSet<String>,
    funcs: BTreeSet<usize>,
    support: usize,
    file: PathBuf,
    line: usize,
}

/// Union-find merge of seeds that share at least `size - 1` fields, collapsing
/// the subset explosion of one recurring bundle into a single group.
fn merge_overlapping(seeds: Vec<(Vec<String>, Seed)>, size: usize) -> Vec<Group> {
    let n = seeds.len();
    let mut parent: Vec<usize> = (0..n).collect();
    let overlap_min = size.saturating_sub(1);
    for i in 0..n {
        for j in (i + 1)..n {
            if shared(&seeds[i].0, &seeds[j].0) >= overlap_min {
                union(&mut parent, i, j);
            }
        }
    }

    let mut by_root: BTreeMap<usize, Group> = BTreeMap::new();
    for (i, (combo, seed)) in seeds.into_iter().enumerate() {
        let root = find(&mut parent, i);
        let entry = by_root.entry(root).or_insert_with(|| Group {
            fields: BTreeSet::new(),
            funcs: BTreeSet::new(),
            support: 0,
            file: seed.file.clone(),
            line: seed.line,
        });
        entry.fields.extend(combo);
        entry.funcs.extend(seed.funcs);
        entry.support = entry.funcs.len();
    }
    by_root.into_values().collect()
}

/// Count of fields two sorted combinations share.
fn shared(a: &[String], b: &[String]) -> usize {
    let set: BTreeSet<&String> = a.iter().collect();
    b.iter().filter(|x| set.contains(x)).count()
}

/// All sorted combinations of `k` items from `items` (assumed already sorted).
fn combinations(items: &[String], k: usize) -> Vec<Vec<String>> {
    let mut out = Vec::new();
    let mut idx: Vec<usize> = (0..k).collect();
    if k == 0 || k > items.len() {
        return out;
    }
    loop {
        out.push(idx.iter().map(|&i| items[i].clone()).collect());
        let mut i = k;
        loop {
            if i == 0 {
                return out;
            }
            i -= 1;
            if idx[i] != i + items.len() - k {
                break;
            }
        }
        idx[i] += 1;
        for j in i + 1..k {
            idx[j] = idx[j - 1] + 1;
        }
    }
}
