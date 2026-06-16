//! A minimal async JSON-RPC 2.0 server speaking the Model Context Protocol over
//! newline-delimited stdio.

mod gate;
mod handlers;
mod prompts;
mod protocol;
mod server;
mod tools;

pub use protocol::handle_message;
pub use server::serve;
