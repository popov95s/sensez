//! The smell pillar fans out over files with rayon; its output order and
//! content must not depend on scheduling. Findings are compared across
//! repeated runs of the same corpus.

use crate::config::smells::Smells;
use crate::spine::parser::parse_file;
use std::fs;

#[test]
fn parallel_detection_is_deterministic() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    // Several files, each tripping a different local smell, so the rayon
    // fan-out has real per-file work to reorder if it could.
    let deep = format!(
        "def deep(a):\n{}{}return a\n",
        (1..=5)
            .map(|i| format!("{}if a > {i}:\n", "    ".repeat(i)))
            .collect::<String>(),
        "    ".repeat(6)
    );
    let long_params = "def wide(a, b, c, d, e, f, g, h):\n    return a\n";
    let magic = "def m(x):\n    y = x * 73 + 1441\n    z = y - 9973\n    w = z / 311\n    return w * 86399\n";
    let chain = "def c(q):\n    return q.a.b.c.d.e.f\n";
    for (name, body) in [
        ("deep.py", deep.as_str()),
        ("wide.py", long_params),
        ("magic.py", magic),
        ("chain.py", chain),
    ] {
        fs::write(dir.join(name), body).unwrap();
    }

    let files: Vec<_> = ["deep.py", "wide.py", "magic.py", "chain.py"]
        .iter()
        .enumerate()
        .map(|(i, n)| parse_file(&dir.join(n), i as u32).unwrap())
        .collect();
    let graph = crate::spine::graph::build(&files, &[]);
    let cfg = Smells::default();

    let runs: Vec<String> = (0..5)
        .map(|_| format!("{:?}", super::detect(&files, &graph, &cfg.clone().into())))
        .collect();
    assert!(
        !runs[0].is_empty() && runs[0].contains("SmellFinding"),
        "corpus must actually produce findings, got none"
    );
    assert!(
        runs.windows(2).all(|w| w[0] == w[1]),
        "smell detection must be deterministic under rayon"
    );
}
