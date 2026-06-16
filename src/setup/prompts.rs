use anyhow::{Context, Result};
use std::io::{BufRead, IsTerminal, Write};

/// True when we can actually ask the user something.
pub fn interactive() -> bool {
    std::io::stdin().is_terminal() && std::io::stderr().is_terminal()
}

/// Numbered single-choice prompt; returns the selected index. Empty input
/// picks `default`.
pub fn select(question: &str, options: &[&str], default: usize) -> Result<usize> {
    let mut err = std::io::stderr();
    writeln!(err, "\n{question}").context("writing prompt")?;
    for (i, option) in options.iter().enumerate() {
        let marker = if i == default { "*" } else { " " };
        writeln!(err, "  {}{} {}", i + 1, marker, option).context("writing prompt")?;
    }
    write!(err, "> ").context("writing prompt")?;
    err.flush().context("flushing prompt")?;

    let answer = read_line()?;
    if answer.is_empty() {
        return Ok(default);
    }
    answer
        .parse::<usize>()
        .ok()
        .and_then(|n| n.checked_sub(1))
        .filter(|i| *i < options.len())
        .ok_or_else(|| anyhow::anyhow!("invalid choice '{answer}'"))
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
