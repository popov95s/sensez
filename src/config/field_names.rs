//! Field names, aliases included, that a derived `Deserialize` struct accepts.

use serde::de::{self, DeserializeOwned, Deserializer, Visitor};
use std::fmt;

/// Every name `T` deserializes a field from; empty when `T` is not a plain derived struct.
pub(super) fn accepted_fields<T: DeserializeOwned>() -> &'static [&'static str] {
    let mut fields: &'static [&'static str] = &[];
    let _ = T::deserialize(FieldProbe {
        fields: &mut fields,
    });
    fields
}

struct FieldProbe<'a> {
    fields: &'a mut &'static [&'static str],
}

#[derive(Debug)]
struct Probed;

impl fmt::Display for Probed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("field probe")
    }
}

impl std::error::Error for Probed {}

impl de::Error for Probed {
    fn custom<T: fmt::Display>(_: T) -> Self {
        Probed
    }
}

impl<'de> Deserializer<'de> for FieldProbe<'_> {
    type Error = Probed;

    fn deserialize_any<V: Visitor<'de>>(self, _: V) -> Result<V::Value, Probed> {
        Err(Probed)
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _: &'static str,
        fields: &'static [&'static str],
        _: V,
    ) -> Result<V::Value, Probed> {
        *self.fields = fields;
        Err(Probed)
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes
        byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct map
        enum identifier ignored_any
    }
}
