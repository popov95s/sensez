use std::path::Path;

/// Module names targeted by `pyproject.toml` entry points.
pub(super) fn entry_modules_from_pyproject(project_root: &Path) -> Vec<String> {
    let text = match std::fs::read_to_string(project_root.join("pyproject.toml")) {
        Ok(text) => text,
        Err(_) => return Vec::new(),
    };
    let value: toml::Value = match toml::from_str(&text) {
        Ok(value) => value,
        Err(_) => return Vec::new(),
    };
    let project = match value.get("project") {
        Some(p) => p,
        None => return Vec::new(),
    };

    let mut modules = Vec::new();
    let mut push = |target: &str| {
        let module = target.split(':').next().unwrap_or(target).trim();
        if !module.is_empty() {
            modules.push(module.to_string());
        }
    };
    for key in ["scripts", "gui-scripts"] {
        if let Some(table) = project.get(key).and_then(toml::Value::as_table) {
            table
                .values()
                .filter_map(toml::Value::as_str)
                .for_each(&mut push);
        }
    }
    if let Some(groups) = project.get("entry-points").and_then(toml::Value::as_table) {
        for group in groups.values().filter_map(toml::Value::as_table) {
            group
                .values()
                .filter_map(toml::Value::as_str)
                .for_each(&mut push);
        }
    }
    modules
}
