use super::loaded_source::{parse_file_with_parser, parse_loaded_source};
use super::parsed_file::{ParseBatch, ParsedFile};
use crate::report::{ScanIssue, ScanStage};
use crate::spine::ir::FileId;
use anyhow::Result;
use rayon::prelude::*;
use std::path::PathBuf;

pub fn parse_files(files: &[PathBuf]) -> ParseBatch {
    let outcomes: Vec<_> = files
        .par_iter()
        .enumerate()
        .map_init(
            tree_sitter::Parser::new,
            |parser, (i, path)| match parse_file_with_parser(path, FileId::from_index(i), parser) {
                Ok(parsed) => Ok(parsed),
                Err(err) => Err(ScanIssue {
                    stage: ScanStage::Parse,
                    file: Some(path.clone()),
                    message: format!("{err:#}"),
                }),
            },
        )
        .collect();

    let mut parsed = Vec::new();
    let mut issues = Vec::new();
    for outcome in outcomes {
        match outcome {
            Ok(file) => parsed.push(file),
            Err(issue) => issues.push(issue),
        }
    }
    ParseBatch {
        files: parsed,
        issues,
    }
}

pub fn parse_sources(files: &[crate::spine::cache::SourceFile]) -> ParseBatch {
    let outcomes: Vec<_> = files
        .par_iter()
        .enumerate()
        .map_init(tree_sitter::Parser::new, |parser, (i, source)| {
            parse_loaded_source(source, FileId::from_index(i), parser).map_err(|err| ScanIssue {
                stage: ScanStage::Parse,
                file: Some(source.path.clone()),
                message: format!("{err:#}"),
            })
        })
        .collect();

    let mut parsed = Vec::new();
    let mut issues = Vec::new();
    for outcome in outcomes {
        match outcome {
            Ok(file) => parsed.push(file),
            Err(issue) => issues.push(issue),
        }
    }
    ParseBatch {
        files: parsed,
        issues,
    }
}

pub fn parse_sources_incremental(
    files: &[crate::spine::cache::SourceFile],
    cache: &mut crate::spine::cache::ParseCacheState,
) -> (ParseBatch, crate::spine::cache::ChangeStats) {
    let stats = cache.changes(files);
    let mut outcomes: Vec<Option<Result<ParsedFile>>> = (0..files.len()).map(|_| None).collect();
    for (index, source) in files.iter().enumerate() {
        if let Some(parsed) = cache.restore(source, FileId::from_index(index)) {
            outcomes[index] = Some(Ok(parsed));
        }
    }
    let misses: Vec<_> = files
        .par_iter()
        .enumerate()
        .filter(|(index, _)| outcomes[*index].is_none())
        .map_init(tree_sitter::Parser::new, |parser, (index, source)| {
            (
                index,
                parse_loaded_source(source, FileId::from_index(index), parser),
            )
        })
        .collect();
    for (index, outcome) in misses {
        outcomes[index] = Some(outcome);
    }

    let mut batch = ParseBatch::default();
    for (source, outcome) in files.iter().zip(outcomes) {
        match outcome {
            Some(Ok(file)) => batch.files.push(file),
            Some(Err(err)) => batch.issues.push(ScanIssue {
                stage: ScanStage::Parse,
                file: Some(source.path.clone()),
                message: format!("{err:#}"),
            }),
            None => unreachable!("every source is restored or parsed"),
        }
    }
    (batch, stats)
}
