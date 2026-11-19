use super::context::SmellContext;
use super::finalize::{apply_actions, apply_rule_actions, fill_spans};
use super::{
    clumps, cohesion, complexity, coupling, dependency_reimplementation, fixture_echo, graphy,
    inherit, loose_fields, mocked_subject, mutation, name_promise, performance, review_risks,
    sibling_helpers, size, structural, stub_shipped, type_escape, typing,
};
use crate::config::smells::{SmellConfig, Smells};
use crate::globs::compile_validated;
use crate::noze::source_scope::SourceScope;
use crate::report::{SmellFinding, SmellKind};
use crate::spine::graph::{CodebaseGraph, ModuleNode};
use crate::spine::ir::FunctionUnit;
use crate::spine::parser::ParsedFile;
use rayon::prelude::*;
use std::collections::HashSet;

/// Detect every enabled smell across the corpus.
pub fn detect(
    files: &[ParsedFile],
    graph: &CodebaseGraph,
    cfg: &SmellConfig,
    scope: &SourceScope,
) -> Vec<SmellFinding> {
    if !cfg.enabled {
        return Vec::new();
    }
    let kept: Vec<&ParsedFile> = files.iter().filter(|f| scope.includes(f)).collect();

    let mut out: Vec<SmellFinding> = kept
        .par_iter()
        .flat_map(|f| detect_local(f, cfg.for_language(f.language)))
        .collect();
    let explicit = compile_validated(&cfg.explicit_exclude);
    out.par_extend(
        files
            .par_iter()
            .filter(|file| !scope.includes(file) && !explicit.is_match(&file.path))
            .flat_map(|file| test_rules(file, cfg.for_language(file.language))),
    );

    let mut cross_file = clumps::detect(&kept, cfg);
    cross_file.extend(sibling_helpers::detect(&kept, graph, cfg));
    cross_file.extend(dependency_reimplementation::detect(&kept, cfg));
    cross_file.extend(graphy::detect(graph, files, cfg, &|node: &ModuleNode| {
        node.source_index
            .and_then(|index| files.get(index))
            .is_some_and(|file| scope.includes(file))
    }));
    apply_rule_actions(&mut cross_file, &kept, cfg);
    out.extend(cross_file);
    out
}

/// The test-source rules on a file the ordinary smell scope skips.
fn test_rules(file: &ParsedFile, settings: &Smells) -> Vec<SmellFinding> {
    let ctx = SmellContext::from_file(file);
    let mut findings = mocked_subject::detect(file, &ctx, settings);
    if settings.is_enabled(SmellKind::WeakTestOracle) {
        findings.extend(review_risks::detect_test_oracles(
            &ctx,
            &file.walked.units.functions,
            settings,
        ));
    }
    fixture_echo::detect(&ctx, &file.walked.units.functions, settings, &mut findings);
    findings.retain(|finding| settings.is_enabled(finding.kind));
    apply_actions(&mut findings, settings);
    fill_spans(file, &mut findings);
    findings
}

pub fn detect_local(file: &ParsedFile, cfg: &Smells) -> Vec<SmellFinding> {
    let ctx = SmellContext::from_file(file);
    let metrics: &[FunctionUnit] = &file.walked.units.functions;

    let mut out = Vec::new();
    let classes = file.walked.units.classes.as_slice();
    let usage = coupling::UsageFacts {
        attribute_accesses: &file.walked.usage.attribute_accesses,
    };
    let locals: HashSet<&str> = classes.iter().map(|c| c.name.as_str()).collect();
    complexity::detect(&ctx, metrics, cfg, &mut out);
    size::detect(&ctx, metrics, cfg, classes, &mut out);
    structural::detect(&ctx, metrics, cfg, &mut out);
    cohesion::detect(&ctx, metrics, classes, &mut out);
    review_risks::detect(&ctx, metrics, classes, cfg, &mut out);
    coupling::detect(&ctx, metrics, &usage, &locals, cfg, &mut out);
    inherit::detect(&ctx, classes, cfg, &mut out);
    stub_shipped::detect(&ctx, metrics, &cfg.stub_markers, &mut out);
    typing::detect(&ctx, metrics, cfg, &mut out);
    type_escape::detect(
        &ctx,
        metrics,
        &file.walked.units.comment_spans,
        cfg,
        &mut out,
    );
    if cfg.is_enabled(SmellKind::LooseTyping) {
        loose_fields::detect(&ctx, classes, cfg, &mut out);
    }
    mutation::detect(&ctx, metrics, cfg, &mut out);
    fixture_echo::detect(&ctx, metrics, cfg, &mut out);
    name_promise::detect(&ctx, metrics, cfg, &mut out);
    out.extend(performance::detect(&ctx, metrics, cfg));
    out.extend(mocked_subject::detect(file, &ctx, cfg));
    out.retain(|f| cfg.is_enabled(f.kind));
    apply_actions(&mut out, cfg);
    fill_spans(file, &mut out);
    out
}
