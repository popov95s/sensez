//! JSONC reading for `tsconfig.json` files.

use serde_json::Value;
use std::path::Path;

/// Parsed JSONC contents of a `tsconfig.json`.
pub(super) fn read_tsconfig(path: &Path) -> Option<Value> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&normalize_jsonc(&text)).ok()
}

enum Jsonc {
    Code,
    Text { escaped: bool },
}

fn normalize_jsonc(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let mut state = Jsonc::Code;
    while let Some(ch) = chars.next() {
        state = match state {
            Jsonc::Text { escaped } => {
                output.push(ch);
                after_text_char(ch, escaped)
            }
            Jsonc::Code => after_code_char(ch, &mut chars, &mut output),
        };
    }
    output
}

fn after_text_char(ch: char, escaped: bool) -> Jsonc {
    if ch == '"' && !escaped {
        return Jsonc::Code;
    }
    Jsonc::Text {
        escaped: ch == '\\' && !escaped,
    }
}

fn after_code_char(
    ch: char,
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
    output: &mut String,
) -> Jsonc {
    match (ch, chars.peek()) {
        ('"', _) => {
            output.push(ch);
            return Jsonc::Text { escaped: false };
        }
        ('/', Some('/')) => {
            chars.next();
            for next in chars.by_ref() {
                if next == '\n' {
                    output.push('\n');
                    break;
                }
            }
        }
        ('/', Some('*')) => {
            chars.next();
            while let Some(next) = chars.next() {
                if next == '*' && chars.peek() == Some(&'/') {
                    chars.next();
                    break;
                }
                if next == '\n' {
                    output.push('\n');
                }
            }
        }
        (',', _) => {
            let keep_before_comma = output.len();
            output.push(ch);
            loop {
                if let Some(next) = chars.next_if(|next| next.is_whitespace()) {
                    output.push(next);
                } else if comment_follows(chars) {
                    let slash = chars.next().unwrap_or('/');
                    after_code_char(slash, chars, output);
                } else {
                    break;
                }
            }
            if matches!(chars.peek(), Some('}' | ']')) {
                output.truncate(keep_before_comma);
            }
        }
        _ => output.push(ch),
    }
    Jsonc::Code
}

fn comment_follows(chars: &std::iter::Peekable<std::str::Chars<'_>>) -> bool {
    let mut ahead = chars.clone();
    ahead.next() == Some('/') && matches!(ahead.next(), Some('/' | '*'))
}

#[cfg(test)]
#[path = "tsconfig_normalize_tests.rs"]
mod normalize_tests;
