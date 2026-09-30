//! Located configuration problems.

use std::fmt;

/// One configuration problem, located in the file it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigProblem {
    pub file: &'static str,
    /// 1-indexed (line, column), when the problem maps to a spot in the file.
    pub position: Option<(usize, usize)>,
    pub message: String,
}

impl fmt::Display for ConfigProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.position {
            Some((line, column)) => write!(f, "{}:{line}:{column}: {}", self.file, self.message),
            None => write!(f, "{}: {}", self.file, self.message),
        }
    }
}

/// Every error that made a configuration unusable.
#[derive(Debug)]
pub struct ConfigErrors(pub Vec<ConfigProblem>);

impl fmt::Display for ConfigErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid configuration ({} problem(s)):", self.0.len())?;
        for problem in &self.0 {
            write!(f, "\n{problem}")?;
        }
        Ok(())
    }
}

impl std::error::Error for ConfigErrors {}
