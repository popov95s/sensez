/// Compile built-in profile globs once, where the conventions are declared.
pub(crate) fn compile_profile_globs(patterns: &[&str]) -> globset::GlobSet {
    let compiled: Vec<String> = patterns
        .iter()
        .map(|pattern| (*pattern).to_string())
        .collect();
    crate::globs::build_globset(&compiled).expect("built-in profile globs must be valid")
}
