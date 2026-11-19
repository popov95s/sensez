//! TypeScript and TSX language profiles.

mod compiler_options;
#[cfg(test)]
#[path = "dead_code_flood_tests.rs"]
mod dead_code_flood_tests;
#[cfg(test)]
#[path = "dead_code_reach_tests.rs"]
mod dead_code_reach_tests;
#[cfg(test)]
#[path = "dead_code_tests.rs"]
mod dead_code_tests;
#[cfg(test)]
#[path = "duplication_tests.rs"]
mod duplication_tests;
mod edge_phase;
#[cfg(test)]
#[path = "graph_target_tests.rs"]
mod graph_target_tests;
mod info;
mod paths;
#[cfg(test)]
#[path = "paths_tests.rs"]
mod paths_tests;
mod tsconfig;
mod type_imports;
#[cfg(test)]
#[path = "type_imports_tests.rs"]
mod type_imports_tests;

#[cfg(test)]
#[path = "default_export_tests.rs"]
mod default_export_tests;

#[cfg(test)]
#[path = "wording_tests.rs"]
mod wording_tests;

#[cfg(test)]
#[path = "mutation_tests.rs"]
mod mutation_tests;

#[cfg(test)]
#[path = "typing_tests.rs"]
mod typing_tests;

#[cfg(test)]
#[path = "nested_function_tests.rs"]
mod nested_function_tests;

#[cfg(test)]
#[path = "tuple_tests.rs"]
mod tuple_tests;

#[cfg(test)]
#[path = "arrow_name_tests.rs"]
mod arrow_name_tests;
mod profile;
mod shared_traits;

pub use profile::{TsProfile, TsxProfile};

#[cfg(test)]
mod tests;
