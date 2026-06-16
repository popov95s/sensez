//! Generic code fact engine.
//!
//! `spine` owns the language-neutral substrate: file discovery, parse dispatch,
//! the intermediate representation, and the dependency graph. Feature modules
//! consume these facts; language adapters live in `profiles`.

pub mod crawler;
pub mod graph;
pub mod ir;
pub mod parser;
