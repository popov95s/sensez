//! Docs generation helpers for the site reference pages.

use anyhow::{Context, Result};
use clap::CommandFactory;
use std::fs;
use std::path::{Path, PathBuf};

const PILLAR_TERMS: [&str; 5] = ["duplication", "dead_code", "cycles", "boundaries", "smells"];

pub fn write_site(root: impl AsRef<Path>) -> Result<()> {
    let root_dir = root.as_ref();
    write_file(
        root_dir.join("docs/reference/cli.md"),
        render_cli_reference()?,
    )?;
    write_file(
        root_dir.join("docs/reference/findings.md"),
        render_findings_reference(),
    )?;
    Ok(())
}

fn render_cli_reference() -> Result<String> {
    let mut out = String::new();
    out.push_str("# CLI Reference\n\n");
    out.push_str(
        "This page is generated from `src/cli/spec.rs`. Regenerate it with `cargo run --all-features --bin docs`.\n\n",
    );
    out.push_str("The hidden compatibility aliases (`sensez scan`, `sensez explain`, `sensez serve`, and `sensez search`) are kept for older automation but are not shown in the standard help output.\n\n");
    write_help_section(&mut out, "`sensez --help`", &[])?;
    write_help_section(&mut out, "`sensez noze --help`", &["noze"])?;
    write_help_section(
        &mut out,
        "`sensez noze explain --help`",
        &["noze", "explain"],
    )?;
    write_help_section(&mut out, "`sensez init --help`", &["init"])?;
    #[cfg(feature = "mcp")]
    {
        write_help_section(&mut out, "`sensez mcp --help`", &["mcp"])?;
        write_help_section(&mut out, "`sensez mcp serve --help`", &["mcp", "serve"])?;
        write_help_section(&mut out, "`sensez brainz --help`", &["brainz"])?;
        write_help_section(
            &mut out,
            "`sensez brainz report --help`",
            &["brainz", "report"],
        )?;
    }
    #[cfg(feature = "eyez")]
    {
        write_help_section(&mut out, "`sensez eyez --help`", &["eyez"])?;
        write_help_section(&mut out, "`sensez eyez search --help`", &["eyez", "search"])?;
    }
    Ok(out)
}

fn render_findings_reference() -> String {
    let mut out = String::new();
    out.push_str("# Finding Reference\n\n");
    out.push_str(
        "This page is generated from `src/noze/glossary.rs` so the human wording stays aligned with the CLI and MCP `explain` output.\n\n",
    );
    out.push_str("## Pillars\n\n");
    for term in PILLAR_TERMS {
        if let Some(entry) = crate::noze::glossary::lookup(term) {
            out.push_str(&format!("### {} (`{}`)\n\n", entry.title, entry.term));
            out.push_str(&entry.explanation);
            out.push_str("\n\n");
        }
    }
    out.push_str("## Smells\n\n");
    for entry in crate::noze::glossary::all()
        .into_iter()
        .filter(|entry| !PILLAR_TERMS.contains(&entry.term.as_str()))
    {
        out.push_str(&format!("### {} (`{}`)\n\n", entry.title, entry.term));
        out.push_str(&entry.explanation);
        out.push_str("\n\n");
    }
    out
}

fn render_help(path: &[&str]) -> Result<String> {
    let mut command = crate::cli::spec::Cli::command();
    let mut current = &mut command;
    for name in path {
        current = current
            .find_subcommand_mut(name)
            .with_context(|| format!("finding subcommand path {}", path.join(" ")))?;
    }
    let mut buf = Vec::new();
    current.write_long_help(&mut buf)?;
    String::from_utf8(buf).context("rendering help as UTF-8")
}

fn write_help_section(out: &mut String, title: &str, path: &[&str]) -> Result<()> {
    out.push_str(&format!("## {title}\n\n"));
    out.push_str("```text\n");
    out.push_str(&render_help(path)?);
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str("```\n\n");
    Ok(())
}

fn write_file(path: PathBuf, content: String) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    let write = match fs::read_to_string(&path) {
        Ok(existing) => existing != content,
        Err(_) => true,
    };
    if write {
        fs::write(&path, content).with_context(|| format!("writing {}", path.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_site_creates_reference_pages() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_site(dir.path()).expect("write docs");
        let cli = fs::read_to_string(dir.path().join("docs/reference/cli.md"))
            .expect("read cli reference");
        let findings = fs::read_to_string(dir.path().join("docs/reference/findings.md"))
            .expect("read findings reference");
        assert!(cli.contains("# CLI Reference"));
        assert!(findings.contains("# Finding Reference"));
    }
}
