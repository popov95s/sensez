//! Join a language's utility table with one file's functions, imports and declared packages.

use super::dependencies::{
    DeclaredPackages, DependencyScope, KnownUtility, Provider, ProviderSource,
};
use crate::spine::ir::{
    FunctionUnit, ImportContext, ProviderOrigin, SymbolFacts, UtilityLookalike, UtilityProvider,
};

/// What one file can already call: declared packages, its scope, and configured in-house packages.
pub struct FileDependencies<'a> {
    pub declared: &'a DeclaredPackages,
    pub scope: DependencyScope,
    pub configured: &'a [String],
}

/// Module-level functions named like a table utility whose provider is present and not imported here.
pub fn lookalikes(
    table: &[KnownUtility],
    functions: &[FunctionUnit],
    symbols: &SymbolFacts,
    dependencies: &FileDependencies,
) -> Vec<UtilityLookalike> {
    functions
        .iter()
        .filter(|function| is_module_level(function, symbols))
        .filter_map(|function| {
            let utility = utility_named(table, function)?;
            let providers = present_providers(utility, &function.name, dependencies);
            let wraps_provider = providers.iter().any(|provider| {
                symbols
                    .imports
                    .iter()
                    .any(|import| imports_module(import, provider))
            });
            (!providers.is_empty() && !wraps_provider).then(|| UtilityLookalike {
                name: function.name.clone(),
                utility: utility.names[0],
                line: function.start_line,
                arity: function.param_names.len(),
                lines: function.end_line + 1 - function.start_line,
                shape: utility.shape,
                providers,
            })
        })
        .collect()
}

/// Whether any module-level function is named like a table utility.
pub fn has_candidates(
    table: &[KnownUtility],
    functions: &[FunctionUnit],
    symbols: &SymbolFacts,
) -> bool {
    functions.iter().any(|function| {
        is_module_level(function, symbols) && utility_named(table, function).is_some()
    })
}

fn utility_named<'t>(
    table: &'t [KnownUtility],
    function: &FunctionUnit,
) -> Option<&'t KnownUtility> {
    table
        .iter()
        .find(|utility| utility.names.contains(&function.name.as_str()))
}

/// A top-level declaration of the file, not a method, local helper or callback.
fn is_module_level(function: &FunctionUnit, symbols: &SymbolFacts) -> bool {
    !function.is_method && symbols.declared_lines.get(&function.name) == Some(&function.start_line)
}

fn present_providers(
    utility: &KnownUtility,
    name: &str,
    dependencies: &FileDependencies,
) -> Vec<UtilityProvider> {
    let listed = utility
        .providers
        .iter()
        .filter_map(|provider| present(provider, dependencies));
    let configured = dependencies
        .configured
        .iter()
        .map(|package| UtilityProvider {
            module: package.clone(),
            function: name.to_string(),
            origin: ProviderOrigin::Configured,
        });
    listed.chain(configured).collect()
}

fn present(provider: &Provider, dependencies: &FileDependencies) -> Option<UtilityProvider> {
    let (module, origin) = match provider.source {
        ProviderSource::StandardLibrary { module } => (module, ProviderOrigin::StandardLibrary),
        ProviderSource::Package { name, module } => (
            module,
            dependencies.declared.origin(name, dependencies.scope)?,
        ),
    };
    Some(UtilityProvider {
        module: module.to_string(),
        function: provider.function.to_string(),
        origin,
    })
}

/// `target` is `module` itself or one of its submodules (`pkg/sub`, `pkg.sub`).
fn imports_module(import: &ImportContext, provider: &UtilityProvider) -> bool {
    import
        .target_module
        .strip_prefix(provider.module.as_str())
        .is_some_and(|rest| rest.is_empty() || rest.starts_with(['/', '.']))
}
