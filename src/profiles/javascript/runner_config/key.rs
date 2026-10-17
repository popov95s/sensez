use super::value::Value;

/// A config key read as a list of strings.
pub enum Key {
    Absent,
    Strings(Vec<String>),
    Unreadable(String),
}

pub fn read(config: &Value, name: &str) -> Key {
    match config.get(name) {
        None | Some(Value::Other) => Key::Absent,
        Some(value) => match value.strings() {
            Ok(strings) => Key::Strings(strings),
            Err(reason) => Key::Unreadable(reason),
        },
    }
}
