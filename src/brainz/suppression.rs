//! Suppress findings that the user or repository config has already accepted.

use super::fingerprint::{self, Print};
use super::triage;
use std::collections::{BTreeMap, HashSet};
use std::path::Path;

pub fn apply_suppressions(root: &Path, report: &mut crate::report::AnalysisReport) {
    let triaged = triage::load(root);
    let accept = crate::config::model::Config::load(root)
        .map(|c| c.accept)
        .unwrap_or_default();
    if triaged.is_empty() && accept.is_empty() {
        return;
    }
    let ignore = triage::ignored_keys(&triaged);
    let Ok(value) = serde_json::to_value(&*report) else {
        return;
    };
    let prints = fingerprint::fingerprints(&value);
    let ctx = Suppress {
        ignore: &ignore,
        accept: &accept,
    };
    retain_allowed(
        &mut report.cycles,
        prints.get(&fingerprint::Namespace::Cycles),
        "cycles",
        &ctx,
    );
    retain_allowed(
        &mut report.dead_code,
        prints.get(&fingerprint::Namespace::DeadCode),
        "dead_code",
        &ctx,
    );
    retain_allowed(
        &mut report.boundaries,
        prints.get(&fingerprint::Namespace::Boundaries),
        "boundaries",
        &ctx,
    );
    retain_allowed(
        &mut report.duplication,
        prints.get(&fingerprint::Namespace::Duplication),
        "duplication",
        &ctx,
    );
    retain_allowed(
        &mut report.smells,
        prints.get(&fingerprint::Namespace::Smells),
        "smells",
        &ctx,
    );
}

struct Suppress<'a> {
    ignore: &'a HashSet<String>,
    accept: &'a BTreeMap<String, Vec<String>>,
}

impl Suppress<'_> {
    fn hides(&self, print: &Print, pillar: &str) -> bool {
        let rendered = print.rendered();
        if self.ignore.contains(&rendered.key) {
            return true;
        }
        let matches = |key: &str| {
            self.accept
                .get(key)
                .is_some_and(|pats| pats.iter().any(|p| rendered.label.contains(p.as_str())))
        };
        matches(pillar) || matches(&rendered.class)
    }
}

fn retain_allowed<T>(
    items: &mut Vec<T>,
    prints: Option<&Vec<Print>>,
    pillar: &str,
    ctx: &Suppress,
) {
    let Some(prints) = prints else {
        return;
    };
    let mut i = 0;
    items.retain(|_| {
        let keep = prints.get(i).map(|p| !ctx.hides(p, pillar)).unwrap_or(true);
        i += 1;
        keep
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brainz::{record_scan, triage_finding, Origin};
    use serde_json::Value;
    use std::fs;
    use std::process::Command;

    #[test]
    fn triaged_finding_is_suppressed_others_kept() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        if !init_repo(root) {
            return;
        }
        fs::write(
            root.join("m.py"),
            "def live():\n    return 0\n\n\ndef orphan():\n    return 1\n\n\ndef orphan_two():\n    return 2\n",
        )
        .unwrap();
        fs::write(root.join("consumer.py"), "from m import live\n\nlive()\n").unwrap();

        let text = crate::scan(root, None, crate::reporter::Format::Json, 0).unwrap();
        let baseline: Value = serde_json::from_str(&text).unwrap();
        record_scan(
            root,
            &baseline,
            std::time::Duration::from_millis(1),
            None,
            Origin::Tool,
        );
        triage_finding(root, "dead_code", "orphan_two", "false_positive", None).unwrap();

        let mut report = crate::analyze_path(root, None, None).unwrap();
        apply_suppressions(root, &mut report);
        let symbols: Vec<&str> = report.dead_code.iter().map(|f| f.symbol.as_str()).collect();
        assert!(
            symbols.contains(&"orphan"),
            "untriaged finding kept: {symbols:?}"
        );
        assert!(
            !symbols.contains(&"orphan_two"),
            "triaged finding suppressed: {symbols:?}"
        );
    }

    fn init_repo(root: &Path) -> bool {
        let initialized = Command::new("git")
            .arg("init")
            .current_dir(root)
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false);
        if !initialized {
            return false;
        }
        fs::write(root.join("base.py"), "print('base')\n").unwrap();
        let added = Command::new("git")
            .args(["add", "."])
            .current_dir(root)
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false);
        added
            && Command::new("git")
                .args([
                    "-c",
                    "user.email=sensez@example.test",
                    "-c",
                    "user.name=Sensez",
                    "commit",
                    "-m",
                    "base",
                ])
                .current_dir(root)
                .output()
                .map(|out| out.status.success())
                .unwrap_or(false)
    }

    #[test]
    fn committed_accept_list_suppresses_by_label() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fs::write(
            root.join("m.py"),
            "def live():\n    return 0\n\n\ndef orphan():\n    return 1\n\n\ndef keeper():\n    return 2\n",
        )
        .unwrap();
        fs::write(root.join("consumer.py"), "from m import live\n\nlive()\n").unwrap();
        fs::write(
            root.join("sensez.toml"),
            "[accept]\ndead_code = [\"m::orphan\"]\n",
        )
        .unwrap();

        let mut report = crate::analyze_path(root, None, None).unwrap();
        apply_suppressions(root, &mut report);
        let symbols: Vec<&str> = report.dead_code.iter().map(|f| f.symbol.as_str()).collect();
        assert!(
            symbols.contains(&"keeper"),
            "unlisted finding kept: {symbols:?}"
        );
        assert!(
            !symbols.contains(&"orphan"),
            "accept-listed finding suppressed: {symbols:?}"
        );
    }
}
