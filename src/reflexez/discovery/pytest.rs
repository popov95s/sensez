use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub fn test_roots(root: &Path) -> Result<Vec<PathBuf>> {
    let configured = pyproject_roots(root)?.or_else(|| ini_roots(root));
    Ok(configured
        .unwrap_or_default()
        .into_iter()
        .map(|path| root.join(path))
        .collect())
}

pub fn is_collected(file: &Path, roots: &[PathBuf]) -> bool {
    roots.is_empty() || roots.iter().any(|root| file.starts_with(root))
}

fn pyproject_roots(root: &Path) -> Result<Option<Vec<String>>> {
    let path = root.join("pyproject.toml");
    if !path.is_file() {
        return Ok(None);
    }
    let source = std::fs::read_to_string(&path)
        .with_context(|| format!("reading pytest configuration at {}", path.display()))?;
    let value: toml::Value = toml::from_str(&source)
        .with_context(|| format!("parsing pytest configuration at {}", path.display()))?;
    let Some(testpaths) = value
        .get("tool")
        .and_then(|value| value.get("pytest"))
        .and_then(|value| value.get("ini_options"))
        .and_then(|value| value.get("testpaths"))
    else {
        return Ok(None);
    };
    Ok(value_roots(testpaths))
}

fn value_roots(value: &toml::Value) -> Option<Vec<String>> {
    if let Some(path) = value.as_str() {
        return Some(split_paths(path));
    }
    value.as_array().map(|paths| {
        paths
            .iter()
            .filter_map(toml::Value::as_str)
            .map(str::to_string)
            .collect()
    })
}

fn ini_roots(root: &Path) -> Option<Vec<String>> {
    parse_ini(&root.join("pytest.ini"), "[pytest]")
        .or_else(|| parse_ini(&root.join("setup.cfg"), "[tool:pytest]"))
        .or_else(|| parse_ini(&root.join("tox.ini"), "[pytest]"))
}

fn parse_ini(path: &Path, expected_section: &str) -> Option<Vec<String>> {
    let source = std::fs::read_to_string(path).ok()?;
    let lines: Vec<_> = source.lines().collect();
    let start = lines
        .iter()
        .position(|line| line.trim() == expected_section)?;
    let section: Vec<_> = lines[start + 1..]
        .iter()
        .take_while(|line| !line.trim().starts_with('['))
        .copied()
        .collect();
    let assignment = section
        .iter()
        .position(|line| line.trim().starts_with("testpaths"))?;
    let inline = section[assignment]
        .split_once('=')
        .map_or("", |(_, value)| value);
    let values: Vec<_> = std::iter::once(inline)
        .chain(
            section[assignment + 1..]
                .iter()
                .take_while(|line| line.starts_with(char::is_whitespace))
                .copied(),
        )
        .flat_map(split_paths)
        .collect();
    (!values.is_empty()).then_some(values)
}

fn split_paths(value: &str) -> Vec<String> {
    value.split_whitespace().map(str::to_string).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_pyproject_testpaths() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(
            root.path().join("pyproject.toml"),
            "[tool.pytest.ini_options]\ntestpaths = [\"tests\"]\n",
        )
        .unwrap();

        let roots = test_roots(root.path()).unwrap();

        assert_eq!(roots, vec![root.path().join("tests")]);
        assert!(is_collected(&root.path().join("tests/test_one.py"), &roots));
        assert!(!is_collected(
            &root.path().join("examples/test_one.py"),
            &roots
        ));
    }

    #[test]
    fn reads_multiline_ini_testpaths() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(
            root.path().join("pytest.ini"),
            "[pytest]\ntestpaths =\n    tests\n    integration\n",
        )
        .unwrap();

        assert_eq!(test_roots(root.path()).unwrap().len(), 2);
    }
}
