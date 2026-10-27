//! CI workflow reading: the `run:` commands under `.github/workflows`.

use std::path::Path;

const WORKFLOWS_DIR: &str = ".github/workflows";

/// Shell commands from every `run:` step of the workflow files under `root`.
pub fn run_commands(root: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(root.join(WORKFLOWS_DIR)) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|entry| is_workflow(&entry.path()))
        .filter_map(|entry| std::fs::read_to_string(entry.path()).ok())
        .flat_map(|text| run_lines(&text))
        .collect()
}

fn is_workflow(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext == "yml" || ext == "yaml")
}

fn run_lines(text: &str) -> Vec<String> {
    let mut commands = Vec::new();
    let mut lines = text.lines().peekable();
    while let Some(line) = lines.next() {
        let key = line.trim_start();
        let step = key.trim_start_matches("- ");
        let Some(value) = step.strip_prefix("run:") else {
            continue;
        };
        let value = value.trim();
        if !value.starts_with(['|', '>']) {
            commands.push(value.to_string());
            continue;
        }
        let key_indent = line.len() - step.len();
        while let Some(body) =
            lines.next_if(|body| body.trim().is_empty() || indent(body) > key_indent)
        {
            commands.push(body.trim().to_string());
        }
    }
    commands
}

fn indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collects_inline_and_block_run_commands() {
        let text = "jobs:\n  build:\n    steps:\n      - uses: actions/checkout@v4\n      - run: node tools/inline.mjs\n      - name: multi\n        run: |\n          npm ci\n          node tools/block.mjs --flag\n        env:\n          X: tools/not-a-command.mjs\n";

        assert_eq!(
            run_lines(text),
            [
                "node tools/inline.mjs",
                "npm ci",
                "node tools/block.mjs --flag"
            ]
        );
    }
}
