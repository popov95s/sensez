use crate::config::model::DeadCode;
use crate::noze::dead_code::detect;
use crate::spine::parser::parse_file;
use std::fs;

fn dead_symbols(sources: &[(&str, &str)]) -> Vec<String> {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("package.json"), "{\"name\":\"x\"}\n").unwrap();
    let files: Vec<_> = sources
        .iter()
        .enumerate()
        .map(|(i, (name, source))| {
            fs::write(dir.join(name), source).unwrap();
            parse_file(&dir.join(name), i as u32).unwrap()
        })
        .collect();
    let cg = crate::spine::graph::build(&files, &[]);
    detect(&cg, &files, &DeadCode::default())
        .iter()
        .map(|f| f.symbol.clone())
        .collect()
}

#[test]
fn default_import_keeps_named_default_export_alive() {
    let dead = dead_symbols(&[
        (
            "src/widget.tsx",
            "export default function Widget() { return null; }\nexport function unusedNamed() { return 1; }\n",
        ),
        (
            "src/page.tsx",
            "import Widget from './widget';\nexport function Page() { return Widget; }\n",
        ),
    ]);
    assert!(!dead.contains(&"Widget".to_string()), "got {dead:?}");
    assert!(dead.contains(&"unusedNamed".to_string()), "got {dead:?}");
}

#[test]
fn lazy_dynamic_import_keeps_default_export_alive() {
    let dead = dead_symbols(&[
        ("src/panel.tsx", "export default class Panel {}\n"),
        (
            "src/page.tsx",
            "const LazyPanel = lazy(() => import('./panel'));\nexport function Page() { return LazyPanel; }\n",
        ),
    ]);
    assert!(!dead.contains(&"Panel".to_string()), "got {dead:?}");
}

#[test]
fn default_import_in_config_keeps_reporter_alive() {
    let dead = dead_symbols(&[
        (
            "src/reporter.ts",
            "export default class Reporter {}\nexport function unusedHelper() { return 1; }\n",
        ),
        (
            "src/app.config.ts",
            "import Reporter from './reporter';\nexport default { reporters: [new Reporter()] };\n",
        ),
    ]);
    assert!(!dead.contains(&"Reporter".to_string()), "got {dead:?}");
    assert!(dead.contains(&"unusedHelper".to_string()), "got {dead:?}");
}
