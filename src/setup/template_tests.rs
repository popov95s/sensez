use super::tests::temp_root;
use super::*;
use std::fs;

#[test]
fn template_commented_defaults_parse_and_match_config_default() {
    let (_tmp, root) = temp_root();
    artifacts::write_config(
        &root,
        artifacts::WriteConfigOptions {
            self_improvement: true,
            into_pyproject: false,
            semantic_duplication: false,
        },
    )
    .unwrap();
    let template = fs::read_to_string(root.join("sensez.toml")).unwrap();
    let uncommented = template
        .lines()
        .map(uncomment_setting)
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(root.join("sensez.toml"), &uncommented).unwrap();

    let (_, warnings) = crate::config::model::Config::load_with_warnings(&root).unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");

    let documented: toml::Value = toml::from_str(&uncommented).unwrap();
    let defaults = toml::Value::try_from(crate::config::model::Config::default()).unwrap();
    let mut checked = Vec::new();
    collect_scalar_mismatches("", &documented, &defaults, &mut checked);
    let mismatches: Vec<_> = checked.iter().filter(|(_, same)| !same).collect();
    assert!(mismatches.is_empty(), "{mismatches:?}");
    assert!(checked.len() >= 20, "{checked:?}");
}

fn uncomment_setting(line: &str) -> &str {
    let Some(rest) = line.strip_prefix("# ") else {
        return line;
    };
    let is_assignment = rest.split_once(" = ").is_some_and(|(key, _)| {
        !key.is_empty()
            && key
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '"' | '/'))
    });
    if rest.starts_with('[') || is_assignment {
        rest
    } else {
        line
    }
}

fn collect_scalar_mismatches(
    path: &str,
    documented: &toml::Value,
    defaults: &toml::Value,
    out: &mut Vec<(String, bool)>,
) {
    let Some(table) = documented.as_table() else {
        return;
    };
    for (key, value) in table {
        let Some(default) = defaults.get(key) else {
            continue;
        };
        let key_path = format!("{path}{key}");
        match value {
            toml::Value::Table(_) => {
                collect_scalar_mismatches(&format!("{key_path}."), value, default, out)
            }
            toml::Value::Array(_) => {}
            _ => out.push((key_path, value == default)),
        }
    }
}
