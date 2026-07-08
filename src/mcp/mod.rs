//! A minimal async JSON-RPC 2.0 server speaking the Model Context Protocol over
//! newline-delimited stdio.

mod compact;
mod gate;
#[cfg(test)]
mod gate_regression_tests;
#[cfg(test)]
mod gate_tests;
mod handlers;
mod prompts;
mod protocol;
mod repeats;
#[cfg(test)]
mod repeats_tests;
mod scan;
mod server;
mod tools;

pub use protocol::handle_message;
pub use server::serve;
