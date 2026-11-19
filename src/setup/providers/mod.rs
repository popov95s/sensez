mod adapter;
mod codex;
mod json_config;
mod opencode;
mod standard;

pub use adapter::McpAdapter;
pub(crate) use json_config::{ensure_object, load_json_config, write_json};

#[cfg(test)]
mod tests;
