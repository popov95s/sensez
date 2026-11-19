//! Rust language profile.

mod calls;
mod chains;
pub(crate) mod comment_syntax;
mod control_shapes;
pub(crate) mod deadcode;
pub(crate) mod imports;
pub(crate) mod lexeme;
mod loops;
mod manifest;
mod nesting;
mod params;
pub(crate) mod performance;
pub(crate) mod resolve;
pub(crate) mod roots;
pub(crate) mod scope;
pub(crate) mod symbols;
pub(crate) mod tokens;
pub(crate) mod traversal;
pub(crate) mod typevocab;
mod unit_helpers;
pub(crate) mod units;
pub(crate) mod wording;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod units_tests;

mod profile;

pub use profile::RustProfile;
