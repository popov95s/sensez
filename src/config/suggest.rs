/// The closest candidate to a misspelled `given`, if any is close enough.
pub(super) fn suggest<'a>(
    given: &str,
    candidates: impl IntoIterator<Item = &'a str>,
) -> Option<&'a str> {
    candidates
        .into_iter()
        .map(|candidate| (strsim::levenshtein(given, candidate), candidate))
        .filter(|(distance, _)| *distance <= 2 && *distance < given.len())
        .min()
        .map(|(_, candidate)| candidate)
}
