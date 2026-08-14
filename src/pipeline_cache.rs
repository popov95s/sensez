//! Cache-specific branches kept out of scan orchestration.

use crate::spine::cache::{ChangeStats, ParseCache, ProjectInputs};
use crate::spine::parser::{ParseBatch, ParsedFile};
use std::path::PathBuf;

use super::PhaseTimer;

pub(super) struct PersistRequest {
    pub is_cli: bool,
    pub project: Option<ProjectInputs>,
    pub parse_cache: Option<ParseCache>,
    pub cacheable: bool,
    pub parse_stats: Option<ChangeStats>,
}

pub(super) fn persist_analysis(request: PersistRequest, parsed_files: &mut Vec<ParsedFile>) {
    if !request.is_cli || !request.cacheable {
        return;
    }
    let (Some(project), Some(parse_cache)) = (request.project, request.parse_cache) else {
        return;
    };
    let refresh = request.parse_stats.is_some_and(|stats| stats.reusable == 0);
    let capture_parse = refresh && ParseCache::worth_capturing(&project.sources);
    let parsed =
        capture_parse.then(|| ParseCache::capture(&project.sources, std::mem::take(parsed_files)));
    crate::spine::cache::persist(parse_cache, parsed, refresh && !capture_parse);
}

pub(super) fn parse_available(
    files: &[PathBuf],
    project: Option<&ProjectInputs>,
    cache: Option<&ParseCache>,
    timer: &mut PhaseTimer,
) -> (ParseBatch, Option<ChangeStats>) {
    if let (Some(project), Some(cache)) = (project, cache) {
        return parse_persisted(project, cache, timer);
    }
    (crate::spine::parser::parse_files(files), None)
}

fn parse_persisted(
    project: &ProjectInputs,
    cache: &ParseCache,
    timer: &mut PhaseTimer,
) -> (ParseBatch, Option<ChangeStats>) {
    let mut state = cache.load();
    let (parsed, stats) =
        crate::spine::parser::parse_sources_incremental(&project.sources, &mut state);
    timer.incremental_cache(stats, project.sources.len());
    (parsed, Some(stats))
}
