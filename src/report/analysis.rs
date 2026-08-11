use super::*;

/// Aggregate result of all pillars.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnalysisReport {
    pub meta: ReportMeta,
    pub cycles: Vec<CycleFinding>,
    pub dead_code: Vec<DeadCodeFinding>,
    pub boundaries: Vec<BoundaryViolation>,
    pub duplication: Vec<CloneClass>,
    pub smells: Vec<SmellFinding>,
}

impl AnalysisReport {
    /// Actions for every finding, in report pillar order.
    pub fn actions(&self) -> impl Iterator<Item = ActionLevel> + '_ {
        self.cycles
            .iter()
            .map(|finding| finding.action)
            .chain(self.dead_code.iter().map(|finding| finding.action))
            .chain(self.boundaries.iter().map(|finding| finding.action))
            .chain(self.duplication.iter().map(|finding| finding.action))
            .chain(self.smells.iter().map(|finding| finding.action))
    }

    pub fn finding_count(&self) -> usize {
        self.actions().count()
    }

    pub fn count_at_action(&self, action: ActionLevel) -> usize {
        self.actions()
            .filter(|candidate| *candidate == action)
            .count()
    }

    /// Hash over the *content* the gate would nag the agent about
    /// (file + line + kind per finding), not over file mtimes. Two
    /// invocations with the same complaint set get the same hash; an
    /// edit that doesn't touch any of the reported files/lines leaves
    /// the hash alone. Used by the `noze_gate` end-of-turn hook to
    /// avoid re-blocking the same unchanged work when an MCP host
    /// (e.g. anything besides Claude Code's CLI) does not set the
    /// `stop_hook_active` flag.
    ///
    /// Pillar tags prefix each section so a dead-code and a smell on
    /// the same file+line don't collide. Line numbers in a file are
    /// not perturbed by edits to other files, so they're stable across
    /// turns; mtimes would not be.
    pub fn finding_signature(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = rustc_hash::FxHasher::default();
        "cycles".hash(&mut h);
        for f in &self.cycles {
            for edge in &f.edges {
                edge.file.hash(&mut h);
                edge.line.hash(&mut h);
            }
        }
        "dead_code".hash(&mut h);
        for f in &self.dead_code {
            f.module.hash(&mut h);
            f.symbol.hash(&mut h);
            f.line.hash(&mut h);
        }
        "boundaries".hash(&mut h);
        for f in &self.boundaries {
            f.from_module.hash(&mut h);
            f.to_module.hash(&mut h);
            f.line.hash(&mut h);
        }
        "duplication".hash(&mut h);
        for f in &self.duplication {
            f.token_length.hash(&mut h);
            for occ in &f.occurrences {
                occ.file.hash(&mut h);
                occ.start_row.hash(&mut h);
            }
        }
        "smells".hash(&mut h);
        for f in &self.smells {
            f.file.hash(&mut h);
            f.line.hash(&mut h);
            f.symbol.hash(&mut h);
            f.kind.hash(&mut h);
        }
        h.finish()
    }

    /// One-line per finding up to `max`, joined by `"; "`. The shape is
    /// `pillar/<kind> <symbol-or-module> <file>:<line>` and is what the
    /// `noze_gate` end-of-turn hook relays to the agent. The order is
    /// dead-code → cycles → boundaries → duplication → smells; findings
    /// are not pre-sorted by impact, so the caller is expected to either
    /// cap the report (via `noze::limit`) first or pass a `max` that
    /// matches the pillar order they want.
    pub fn top_n_summary(&self, max: usize) -> String {
        let mut items = Vec::new();
        items.extend(self.dead_code.iter().map(|f| {
            format!(
                "dead_code/{} {}::{} {}:{}",
                f.kind,
                f.module,
                f.symbol,
                f.file.display(),
                f.line
            )
        }));
        items.extend(
            self.cycles
                .iter()
                .map(|f| format!("cycle {}", f.modules.join(" -> "))),
        );
        items.extend(self.boundaries.iter().map(|f| {
            format!(
                "boundary {} -> {} {}:{}",
                f.from_module,
                f.to_module,
                f.file.display(),
                f.line
            )
        }));
        items.extend(self.duplication.iter().map(|f| {
            let locations = f
                .occurrences
                .iter()
                .map(|o| format!("{}:{}", o.file.display(), o.start_row))
                .collect::<Vec<_>>()
                .join(", ");
            format!("duplication {} token(s) at {locations}", f.token_length)
        }));
        items.extend(self.smells.iter().map(|f| {
            format!(
                "smell/{} {} {}:{}",
                f.kind,
                f.symbol,
                f.file.display(),
                f.line
            )
        }));
        items.truncate(max);
        items.join("; ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aggregates_actions_across_findings() {
        let report = AnalysisReport {
            duplication: vec![
                CloneClass {
                    action: ActionLevel::Warning,
                    token_length: 10,
                    occurrences: Vec::new(),
                    hint: None,
                },
                CloneClass {
                    action: ActionLevel::MustFix,
                    token_length: 20,
                    occurrences: Vec::new(),
                    hint: None,
                },
            ],
            ..AnalysisReport::default()
        };

        assert_eq!(report.finding_count(), 2);
        assert_eq!(report.count_at_action(ActionLevel::MustFix), 1);
        assert_eq!(
            report.actions().collect::<Vec<_>>(),
            vec![ActionLevel::Warning, ActionLevel::MustFix]
        );
    }
}
