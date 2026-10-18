//! JSONC reading for `tsconfig.json` files.

use serde_json::Value;
use std::path::Path;

/// Parsed JSONC contents of a `tsconfig.json`.
pub(super) fn read_tsconfig(path: &Path) -> Option<Value> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&normalize_jsonc(&text)).ok()
}

fn normalize_jsonc(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let mut state = JsoncState::default();
    while let Some(ch) = chars.next() {
        if state.push_string_char(ch, &mut output) {
            continue;
        }
        if ch == '"' {
            state.start_string();
            output.push(ch);
            continue;
        }
        if ch == '/' && chars.peek() == Some(&'/') {
            chars.next();
            for next in chars.by_ref() {
                if next == '\n' {
                    output.push('\n');
                    break;
                }
            }
            continue;
        }
        if ch == '/' && chars.peek() == Some(&'*') {
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
            continue;
        }
        if ch == ',' {
            let keep_before_comma = output.len();
            output.push(ch);
            while let Some(next) = chars.next_if(|next| next.is_whitespace()) {
                output.push(next);
            }
            if matches!(chars.peek(), Some('}' | ']')) {
                output.truncate(keep_before_comma);
            }
            continue;
        }
        output.push(ch);
    }
    output
}

#[derive(Default)]
struct JsoncState {
    in_string: bool,
    escaped: bool,
}

impl JsoncState {
    fn start_string(&mut self) {
        self.in_string = true;
        self.escaped = false;
    }

    fn push_string_char(&mut self, ch: char, output: &mut String) -> bool {
        if !self.in_string {
            return false;
        }
        let escaped = self.escaped;
        self.escaped = ch == '\\' && !escaped;
        if ch == '"' && !escaped {
            self.in_string = false;
        }
        output.push(ch);
        true
    }
}

#[cfg(test)]
mod normalize_tests {
    use super::normalize_jsonc;

    #[test]
    fn preserves_escaped_quotes_and_comment_markers_in_strings() {
        let input = r#"{ "label": "escaped \" // text", "trailing": 1, }"#;
        let normalized = normalize_jsonc(input);
        let value: serde_json::Value = serde_json::from_str(&normalized).unwrap();
        assert_eq!(value["label"], "escaped \" // text");
        assert_eq!(value["trailing"], 1);
    }

    #[test]
    fn strips_comments_and_trailing_commas_into_parseable_json() {
        let input = "{ /* c */ \"a\": [1, 2,], // line\n \"b\": { \"c\": 1, }, }";
        let normalized = normalize_jsonc(input);
        let value: serde_json::Value = serde_json::from_str(&normalized).unwrap_or_else(|err| {
            panic!("normalized output must be valid JSON: {err}\n{normalized}")
        });
        assert_eq!(value["a"], serde_json::json!([1, 2]));
        assert_eq!(value["b"]["c"], 1);
        assert!(!normalized.contains("//"), "line comments must be stripped");
    }
}
