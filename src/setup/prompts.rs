use anyhow::{Context, Result};
use dialoguer::{console::Term, theme::ColorfulTheme, Select};
use std::io::{BufRead, IsTerminal, Write};

/// True when we can actually ask the user something.
pub fn interactive() -> bool {
    std::io::stdin().is_terminal() && std::io::stderr().is_terminal()
}

/// Single-choice prompt with arrow-key navigation.
pub fn select(question: &str, options: &[&str], default: usize) -> Result<usize> {
    if !interactive() {
        return Err(anyhow::anyhow!("interactive selection requires a TTY"));
    }
    Select::with_theme(&ColorfulTheme::default())
        .with_prompt(question)
        .items(options)
        .default(default.min(options.len().saturating_sub(1)))
        .interact_on_opt(&Term::stderr())
        .context("showing selection prompt")?
        .ok_or_else(|| anyhow::anyhow!("selection cancelled"))
}

/// Yes/no prompt; empty input picks `default`.
pub fn confirm(question: &str, default: bool) -> Result<bool> {
    let mut err = std::io::stderr();
    let hint = if default { "[Y/n]" } else { "[y/N]" };
    write!(err, "\n{question} {hint} ").context("writing prompt")?;
    err.flush().context("flushing prompt")?;

    let answer = read_line()?.to_lowercase();
    Ok(match answer.as_str() {
        "" => default,
        "y" | "yes" => true,
        "n" | "no" => false,
        other => anyhow::bail!("expected y or n, got '{other}'"),
    })
}

fn read_line() -> Result<String> {
    let mut line = String::new();
    std::io::stdin()
        .lock()
        .read_line(&mut line)
        .context("reading answer")?;
    Ok(line.trim().to_string())
}
