use crate::spine::parser::FunctionUnit;
use rustc_hash::FxHashSet;

/// Words too common in names to say anything about what a function is for.
const GENERIC_WORDS: &[&str] = &[
    "an", "and", "by", "get", "is", "it", "of", "or", "set", "the", "to", "tmp",
];

/// Lowercase words of the function's name and parameter names, in order.
pub(super) fn signature_words(func: &FunctionUnit) -> Vec<String> {
    let mut seen = FxHashSet::default();
    std::iter::once(func.name.as_str())
        .chain(func.param_names.iter().map(String::as_str))
        .flat_map(split_identifier)
        .filter(|word| word.len() > 1 && !GENERIC_WORDS.contains(&word.as_str()))
        .filter(|word| seen.insert(word.clone()))
        .collect()
}

/// `parseCsvRow`, `parse_csv_row` and `HTTPServer` split into lowercase words.
pub(super) fn split_identifier(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut words = Vec::new();
    let mut current = String::new();
    for (index, &c) in chars.iter().enumerate() {
        if !c.is_alphanumeric() {
            flush(&mut words, &mut current);
            continue;
        }
        if starts_word(&chars, index) {
            flush(&mut words, &mut current);
        }
        current.extend(c.to_lowercase());
    }
    flush(&mut words, &mut current);
    words
}

fn starts_word(chars: &[char], index: usize) -> bool {
    let c = chars[index];
    let Some(&previous) = index.checked_sub(1).and_then(|p| chars.get(p)) else {
        return false;
    };
    let next_is_lower = chars.get(index + 1).is_some_and(|next| next.is_lowercase());
    c.is_uppercase()
        && (previous.is_lowercase()
            || previous.is_ascii_digit()
            || (previous.is_uppercase() && next_is_lower))
}

fn flush(words: &mut Vec<String>, current: &mut String) {
    if !current.is_empty() {
        words.push(std::mem::take(current));
    }
}

#[cfg(test)]
#[path = "intent_tests.rs"]
mod tests;
