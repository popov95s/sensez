//! Runner adapters build ordinary commands; test collection/execution stays native.

mod execute;
mod plans;

pub use execute::execute;
pub use plans::plans;

#[cfg(test)]
mod tests;
