//! Changed-code test impact analysis that delegates execution to existing runners.

mod assets;
mod changes;
mod discovery;
mod dynamic;
mod model;
mod module_root;
mod patterns;
mod reach;
mod render;
mod run;
mod runner_config;
mod runners;
mod selector;
mod selector_dynamic;
mod setup_trigger;

pub(crate) use run::run;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_changes;
#[cfg(test)]
mod tests_config;
#[cfg(test)]
mod tests_declaration;
#[cfg(test)]
mod tests_discovery;
#[cfg(test)]
mod tests_graph;
#[cfg(test)]
mod tests_settings;
