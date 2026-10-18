//! Symbol-level evidence used to decide dead-code candidates.

use super::demand::Demand;
use crate::profiles::DeadCodeProfile;
use crate::spine::parser::ParsedFile;

/// Cheap, decoration-independent reasons a symbol is not a dead candidate.
pub(super) fn skip_symbol(
    file: &ParsedFile,
    profile: &dyn DeadCodeProfile,
    symbol: &str,
    used: &Demand,
) -> bool {
    if profile.is_conventionally_private(symbol) {
        return true;
    }
    if file
        .walked
        .symbols
        .dunder_all
        .as_ref()
        .is_some_and(|all| all.iter().any(|s| s == symbol))
    {
        return true;
    }
    if file
        .walked
        .usage
        .name_counts
        .get(symbol)
        .copied()
        .unwrap_or(0)
        > 1
    {
        return true; // referenced within its own module
    }
    is_asked(file, symbol, used)
}

/// `demand` names `symbol`, directly or as the module's default export.
pub(super) fn is_asked(file: &ParsedFile, symbol: &str, demand: &Demand) -> bool {
    let is_default = file.walked.symbols.default_export.as_deref() == Some(symbol);
    demand.contains(symbol) || (is_default && demand.contains("default"))
}
