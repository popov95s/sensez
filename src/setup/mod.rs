mod agents;
mod artifacts;
mod command;
mod gate;
mod init;
mod prompts;
mod providers;
mod scope;
mod skills;

#[cfg(feature = "mcp")]
pub(crate) use gate::MCP_REQUEST_TIMEOUT_SECS;

#[cfg(test)]
use init::run_with_home;
pub use init::{run, InitOptions};

#[cfg(test)]
#[path = "global_tests.rs"]
mod global_tests;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
#[path = "template_tests.rs"]
mod template_tests;

#[cfg(test)]
#[path = "mcp_config_tests.rs"]
mod mcp_config_tests;

#[cfg(test)]
#[path = "mcp_command_tests.rs"]
mod mcp_command_tests;
