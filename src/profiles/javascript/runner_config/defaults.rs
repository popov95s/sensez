use super::value::{Object, Value};

pub const VITEST_INCLUDE: [&str; 1] = ["**/*.{test,spec}.?(c|m)[jt]s?(x)"];
pub const VITEST_EXCLUDE: [&str; 5] = [
    "**/node_modules/**",
    "**/dist/**",
    "**/cypress/**",
    "**/.{idea,git,cache,output,temp}/**",
    "**/{karma,rollup,webpack,vite,vitest,jest,ava,babel,nyc,cypress,tsup,build,eslint,prettier}.config.*",
];
pub const JEST_TEST_MATCH: [&str; 2] = [
    "**/__tests__/**/*.?([mc])[jt]s?(x)",
    "**/?(*.)+(spec|test).?([mc])[jt]s?(x)",
];
pub const JEST_IGNORE: [&str; 1] = ["/node_modules/"];

/// Values of the defaults `vitest/config` exports.
pub fn vitest_export(name: &str) -> Option<Value> {
    match name {
        "defaultInclude" => Some(strings(&VITEST_INCLUDE)),
        "defaultExclude" => Some(strings(&VITEST_EXCLUDE)),
        "configDefaults" => Some(Value::Object(Object {
            entries: vec![
                ("include".into(), strings(&VITEST_INCLUDE)),
                ("exclude".into(), strings(&VITEST_EXCLUDE)),
            ],
            unknown: None,
        })),
        _ => None,
    }
}

fn strings(values: &[&str]) -> Value {
    Value::Array(owned(values).into_iter().map(Value::Str).collect())
}

pub fn owned(patterns: &[&str]) -> Vec<String> {
    patterns.iter().map(|pattern| pattern.to_string()).collect()
}
