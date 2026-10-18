//! Numeric literals that may need a name, with the context config can waive.

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MagicContext {
    Plain,
    ArrayIndex,
    DefaultValue,
    HttpStatus,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MagicNumber {
    pub line: usize,
    pub text: String,
    pub context: MagicContext,
}
