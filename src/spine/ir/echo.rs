//! Test assertions whose two sides cannot differ.

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EchoShape {
    SameExpression,
    Literals,
    PassThrough,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EchoAssertion {
    pub line: usize,
    pub shape: EchoShape,
}
