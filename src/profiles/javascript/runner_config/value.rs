/// A statically evaluated JavaScript/JSON config value.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Str(String),
    Regex(String),
    Bool(bool),
    Array(Vec<Value>),
    Object(Object),
    Other,
    Opaque(String),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Object {
    pub entries: Vec<(String, Value)>,
    /// Why keys may exist that were not read, e.g. a spread of a call result.
    pub unknown: Option<String>,
}

impl Value {
    /// `None` when the key is known to be absent.
    pub fn get(&self, key: &str) -> Option<Value> {
        match self {
            Self::Object(object) => object
                .entries
                .iter()
                .rev()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value.clone())
                .or_else(|| object.unknown.clone().map(Self::Opaque)),
            Self::Opaque(reason) => Some(Self::Opaque(reason.clone())),
            _ => None,
        }
    }

    /// Copy of an object without `key`.
    pub fn without(&self, key: &str) -> Self {
        match self {
            Self::Object(object) => Self::Object(Object {
                entries: object
                    .entries
                    .iter()
                    .filter(|(name, _)| name != key)
                    .cloned()
                    .collect(),
                unknown: object.unknown.clone(),
            }),
            other => other.clone(),
        }
    }

    pub fn strings(&self) -> Result<Vec<String>, String> {
        match self {
            Self::Str(text) | Self::Regex(text) => Ok(vec![text.clone()]),
            Self::Array(items) => items.iter().try_fold(Vec::new(), |mut out, item| {
                out.extend(item.strings()?);
                Ok(out)
            }),
            Self::Opaque(reason) => Err(reason.clone()),
            _ => Err("value is not a string or an array of strings".into()),
        }
    }

    pub fn from_json(json: &serde_json::Value) -> Self {
        match json {
            serde_json::Value::String(text) => Self::Str(text.clone()),
            serde_json::Value::Bool(flag) => Self::Bool(*flag),
            serde_json::Value::Array(items) => {
                Self::Array(items.iter().map(Self::from_json).collect())
            }
            serde_json::Value::Object(map) => Self::Object(Object {
                entries: map
                    .iter()
                    .map(|(key, value)| (key.clone(), Self::from_json(value)))
                    .collect(),
                unknown: None,
            }),
            _ => Self::Other,
        }
    }
}

/// Vite `mergeConfig` semantics: objects merge per key, arrays concatenate, else `overrides` wins.
pub fn merge(defaults: Value, overrides: Value) -> Value {
    match (defaults, overrides) {
        (Value::Object(mut base), Value::Object(top)) => {
            for (key, value) in top.entries {
                match base.entries.iter().position(|(name, _)| *name == key) {
                    Some(index) => {
                        let current = std::mem::replace(&mut base.entries[index].1, Value::Other);
                        base.entries[index].1 = merge(current, value);
                    }
                    None => base.entries.push((key, value)),
                }
            }
            base.unknown = base.unknown.or(top.unknown);
            Value::Object(base)
        }
        (Value::Array(mut base), Value::Array(top)) => {
            base.extend(top);
            Value::Array(base)
        }
        (_, overrides) => overrides,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn object(entries: Vec<(&str, Value)>) -> Value {
        Value::Object(Object {
            entries: entries
                .into_iter()
                .map(|(key, value)| (key.to_string(), value))
                .collect(),
            unknown: None,
        })
    }

    #[test]
    fn merge_concatenates_arrays_and_merges_objects() {
        let base = object(vec![(
            "test",
            object(vec![(
                "setupFiles",
                Value::Array(vec![Value::Str("a".into())]),
            )]),
        )]);
        let top = object(vec![(
            "test",
            object(vec![
                ("setupFiles", Value::Array(vec![Value::Str("b".into())])),
                ("include", Value::Str("x".into())),
            ]),
        )]);

        let merged = merge(base, top);
        let test = merged.get("test").unwrap();

        assert_eq!(
            test.get("setupFiles").unwrap().strings().unwrap(),
            ["a", "b"]
        );
        assert_eq!(test.get("include").unwrap().strings().unwrap(), ["x"]);
    }

    #[test]
    fn unknown_spread_makes_missing_keys_opaque() {
        let value = Value::Object(Object {
            entries: Vec::new(),
            unknown: Some("spread of `shared()`".into()),
        });

        assert!(matches!(value.get("include"), Some(Value::Opaque(_))));
    }
}
