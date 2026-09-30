use crate::report::ActionLevel;
use crate::spine::parser::parse_file;
use std::fs;

const VERBATIM: &str = r#"{ "compilerOptions": { "verbatimModuleSyntax": true, } }"#;

fn cycle_actions(x_import: &str, configs: &[(&str, &str)]) -> Vec<ActionLevel> {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("package.json"), "{\"name\":\"x\"}\n").unwrap();
    for (name, config) in configs {
        fs::write(dir.join(name), config).unwrap();
    }
    fs::write(
        dir.join("src/x.ts"),
        format!("{x_import}\nexport class X {{}}\n"),
    )
    .unwrap();
    fs::write(
        dir.join("src/y.ts"),
        "import { X } from './x';\nexport class Y { x = new X(); }\nexport const z = 1;\n",
    )
    .unwrap();
    let files: Vec<_> = ["src/x.ts", "src/y.ts"]
        .iter()
        .enumerate()
        .map(|(i, name)| parse_file(&dir.join(name), i as u32).unwrap())
        .collect();
    let cg = crate::spine::graph::build(&files, &[]);
    crate::noze::cycles::detect(&cg, &[])
        .into_iter()
        .map(|finding| finding.action)
        .collect()
}

#[test]
fn inline_type_only_import_cycle_is_info_by_default() {
    assert_eq!(
        cycle_actions("import { type Y } from './y';", &[]),
        vec![ActionLevel::Info]
    );
}

#[test]
fn inline_type_only_reexport_cycle_is_info_by_default() {
    assert_eq!(
        cycle_actions("export { type Y } from './y';", &[]),
        vec![ActionLevel::Info]
    );
}

#[test]
fn verbatim_module_syntax_keeps_inline_type_only_import() {
    assert_eq!(
        cycle_actions(
            "import { type Y } from './y';",
            &[("tsconfig.json", VERBATIM)]
        ),
        vec![ActionLevel::Warning]
    );
}

#[test]
fn verbatim_module_syntax_is_inherited_through_extends() {
    assert_eq!(
        cycle_actions(
            "import { type Y } from './y';",
            &[
                ("tsconfig.base.json", VERBATIM),
                ("tsconfig.json", r#"{ "extends": "./tsconfig.base" }"#),
            ]
        ),
        vec![ActionLevel::Warning]
    );
}

#[test]
fn mixed_inline_type_import_stays_runtime() {
    assert_eq!(
        cycle_actions("import { type Y, z } from './y';", &[]),
        vec![ActionLevel::Warning]
    );
}

#[test]
fn import_type_stays_type_only_under_verbatim_module_syntax() {
    assert_eq!(
        cycle_actions(
            "import type { Y } from './y';",
            &[("tsconfig.json", VERBATIM)]
        ),
        vec![ActionLevel::Info]
    );
}
